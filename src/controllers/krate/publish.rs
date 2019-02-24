//! Functionality related to publishing a new crate or version of a crate.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::offset::TimeZone;
use chrono::{DateTime, Utc};
use hex::ToHex;

use crate::git;
use crate::render;
use crate::util::{internal, ChainError, Maximums};
use crate::util::{read_fill, read_le_u32};

use crate::controllers::prelude::*;
use crate::models::dependency;
use crate::models::{Badge, Category, Keyword, NewCrate, NewVersion, Rights, User};
use crate::views::{EncodableCrateUpload, GoodCrate, PublishWarnings};

/// Handles the `PUT /crates/new` route.
/// Used by `cargo publish` to publish a new crate or to publish a new version of an
/// existing crate.
///
/// Currently blocks the HTTP thread, perhaps some function calls can spawn new
/// threads and return completion or error through other methods  a `cargo publish
/// --status` command, via crates.io's front end, or email.
pub fn publish(req: &mut dyn Request) -> CargoResult<Response> {
    let app = Arc::clone(req.app());

    // The format of the req.body() of a publish request is as follows:
    //
    // metadata length
    // metadata in JSON about the crate being published
    // .crate tarball length
    // .crate tarball file
    //
    // - The metadata is read and interpreted in the parse_new_headers function.
    // - The .crate tarball length is read in this function in order to save the size of the file
    //   in the version record in the database.
    // - Then the .crate tarball length is passed to the upload_crate function where the actual
    //   file is read and uploaded.

    let (new_crate, user) = parse_new_headers(req)?;

    let name = &*new_crate.name;
    let vers = &*new_crate.vers;
    let links = new_crate.links.clone();
    let repo = new_crate.repository.as_ref().map(|s| &**s);
    let features = new_crate
        .features
        .iter()
        .map(|(k, v)| {
            (
                k[..].to_string(),
                v.iter().map(|v| v[..].to_string()).collect(),
            )
        })
        .collect::<HashMap<String, Vec<String>>>();
    let keywords = new_crate
        .keywords
        .as_ref()
        .map(|kws| kws.iter().map(|kw| &***kw).collect())
        .unwrap_or_else(Vec::new);

    let categories = new_crate.categories.as_ref().map(|s| &s[..]).unwrap_or(&[]);
    let categories: Vec<_> = categories.iter().map(|k| &***k).collect();

    let conn = app.diesel_database.get()?;

    let mut other_warnings = vec![];
    let verified_email_address = user.verified_email(&conn)?;

    // This function can be inlined (with only the error-returning functionality) and its unit
    // tests deleted after 2019-02-28; it was created to make injecting the date for tests easier.
    // The integration tests in src/tests/krate.rs cover the current production behavior (and will
    // need to be updated at that time)
    verified_email_check(&mut other_warnings, &verified_email_address, Utc::now())?;

    // Create a transaction on the database, if there are no errors,
    // commit the transactions to record a new or updated crate.
    conn.transaction(|| {
        // Persist the new crate, if it doesn't already exist
        let persist = NewCrate {
            name,
            description: new_crate.description.as_ref().map(|s| &**s),
            homepage: new_crate.homepage.as_ref().map(|s| &**s),
            documentation: new_crate.documentation.as_ref().map(|s| &**s),
            readme: new_crate.readme.as_ref().map(|s| &**s),
            readme_file: new_crate.readme_file.as_ref().map(|s| &**s),
            repository: repo,
            license: new_crate.license.as_ref().map(|s| &**s),
            max_upload_size: None,
        };

        let license_file = new_crate.license_file.as_ref().map(|s| &**s);
        let krate = persist.create_or_update(&conn, license_file, user.id)?;

        let owners = krate.owners(&conn)?;
        if user.rights(req.app(), &owners)? < Rights::Publish {
            return Err(human(
                "this crate exists but you don't seem to be an owner. \
                 If you believe this is a mistake, perhaps you need \
                 to accept an invitation to be an owner before \
                 publishing.",
            ));
        }

        if &krate.name != name {
            return Err(human(&format_args!(
                "crate was previously named `{}`",
                krate.name
            )));
        }

        // Length of the .crate tarball, which appears after the metadata in the request body.
        // TODO: Not sure why we're using the total content length (metadata + .crate file length)
        // to compare against the max upload size... investigate that and perhaps change to use
        // this file length.
        let file_length = read_le_u32(req.body())?;

        let content_length = req
            .content_length()
            .chain_error(|| human("missing header: Content-Length"))?;

        let maximums = Maximums::new(
            krate.max_upload_size,
            app.config.max_upload_size,
            app.config.max_unpack_size,
        );

        if content_length > maximums.max_upload_size {
            return Err(human(&format_args!(
                "max upload size is: {}",
                maximums.max_upload_size
            )));
        }

        // This is only redundant for now. Eventually the duplication will be removed.
        let license = new_crate.license.clone();

        // Persist the new version of this crate
        let version = NewVersion::new(
            krate.id,
            vers,
            &features,
            license,
            license_file,
            // Downcast is okay because the file length must be less than the max upload size
            // to get here, and max upload sizes are way less than i32 max
            file_length as i32,
            user.id,
        )?
        .save(&conn, &new_crate.authors, verified_email_address)?;

        // Link this new version to all dependencies
        let git_deps = dependency::add_dependencies(&conn, &new_crate.deps, version.id)?;

        // Update all keywords for this crate
        Keyword::update_crate(&conn, &krate, &keywords)?;

        // Update all categories for this crate, collecting any invalid categories
        // in order to be able to warn about them
        let ignored_invalid_categories = Category::update_crate(&conn, &krate, &categories)?;

        // Update all badges for this crate, collecting any invalid badges in
        // order to be able to warn about them
        let ignored_invalid_badges = Badge::update_crate(&conn, &krate, new_crate.badges.as_ref())?;
        let max_version = krate.max_version(&conn)?;

        // Render the README for this crate
        let readme = match new_crate.readme.as_ref() {
            Some(readme) => Some(render::readme_to_html(
                &**readme,
                new_crate.readme_file.as_ref().map_or("README.md", |s| &**s),
                repo,
            )?),
            None => None,
        };

        // Upload the crate, return way to delete the crate from the server
        // If the git commands fail below, we shouldn't keep the crate on the
        // server.
        let (cksum, mut crate_bomb, mut readme_bomb) = app
            .config
            .uploader
            .upload_crate(req, &krate, readme, maximums, vers)?;
        version.record_readme_rendering(&conn)?;

        let mut hex_cksum = String::new();
        cksum.write_hex(&mut hex_cksum)?;

        // Register this crate in our local git repo.
        let git_crate = git::Crate {
            name: name.to_string(),
            vers: vers.to_string(),
            cksum: hex_cksum,
            features,
            deps: git_deps,
            yanked: Some(false),
            links,
        };
        git::add_crate(&**req.app(), &git_crate).chain_error(|| {
            internal(&format_args!(
                "could not add crate `{}` to the git repo",
                name
            ))
        })?;

        // Now that we've come this far, we're committed!
        crate_bomb.path = None;
        readme_bomb.path = None;

        let warnings = PublishWarnings {
            invalid_categories: ignored_invalid_categories,
            invalid_badges: ignored_invalid_badges,
            other: other_warnings,
        };

        Ok(req.json(&GoodCrate {
            krate: krate.minimal_encodable(&max_version, None, false, None),
            warnings,
        }))
    })
}

/// Used by the `krate::new` function.
///
/// This function parses the JSON headers to interpret the data and validates
/// the data during and after the parsing. Returns crate metadata and user
/// information.
fn parse_new_headers(req: &mut dyn Request) -> CargoResult<(EncodableCrateUpload, User)> {
    // Read the json upload request
    let metadata_length = u64::from(read_le_u32(req.body())?);
    let max = req.app().config.max_upload_size;
    if metadata_length > max {
        return Err(human(&format_args!("max upload size is: {}", max)));
    }
    let mut json = vec![0; metadata_length as usize];
    read_fill(req.body(), &mut json)?;
    let json = String::from_utf8(json).map_err(|_| human("json body was not valid utf-8"))?;
    let new: EncodableCrateUpload = serde_json::from_str(&json)
        .map_err(|e| human(&format_args!("invalid upload request: {}", e)))?;

    // Make sure required fields are provided
    fn empty(s: Option<&String>) -> bool {
        s.map_or(true, |s| s.is_empty())
    }
    let mut missing = Vec::new();

    if empty(new.description.as_ref()) {
        missing.push("description");
    }
    if empty(new.license.as_ref()) && empty(new.license_file.as_ref()) {
        missing.push("license");
    }
    if new.authors.iter().all(|s| s.is_empty()) {
        missing.push("authors");
    }
    if !missing.is_empty() {
        return Err(human(&format_args!(
            "missing or empty metadata fields: {}. Please \
             see https://doc.rust-lang.org/cargo/reference/manifest.html for \
             how to upload metadata",
            missing.join(", ")
        )));
    }

    let user = req.user()?;
    Ok((new, user.clone()))
}

fn verified_email_check(
    other_warnings: &mut Vec<String>,
    verified_email_address: &Option<String>,
    now: DateTime<Utc>,
) -> CargoResult<()> {
    match verified_email_address {
        Some(_) => Ok(()),
        None => {
            if now < Utc.ymd(2019, 3, 1).and_hms(0, 0, 0) {
                other_warnings.push(String::from(
                    "You do not currently have a verified email address associated with your \
                     crates.io account. Starting 2019-02-28, a verified email will be required to \
                     publish crates. Visit https://crates.io/me to set and verify your email \
                     address.",
                ));
                Ok(())
            } else {
                Err(human(
                    "A verified email address is required to publish crates to crates.io. \
                     Visit https://crates.io/me to set and verify your email address.",
                ))
            }
        }
    }
}

// These tests should be deleted after 2018-02-28; this functionality will then be covered by
// integration tests in src/tests/krate.rs.
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::offset::TimeZone;

    #[test]
    fn allow_publish_with_verified_email_without_warning_before_2018_02_28() {
        let mut warnings = vec![];

        let fake_current_date = Utc.ymd(2019, 2, 27).and_hms(0, 0, 0);
        let result = verified_email_check(
            &mut warnings,
            &Some("someone@example.com".into()),
            fake_current_date,
        );

        assert!(result.is_ok());
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn allow_publish_with_verified_email_without_error_after_2018_02_28() {
        let mut warnings = vec![];

        let fake_current_date = Utc.ymd(2019, 3, 1).and_hms(0, 0, 0);
        let result = verified_email_check(
            &mut warnings,
            &Some("someone@example.com".into()),
            fake_current_date,
        );

        assert!(result.is_ok());
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn warn_without_verified_email_before_2018_02_28() {
        let mut warnings = vec![];

        let fake_current_date = Utc.ymd(2019, 2, 27).and_hms(0, 0, 0);
        let result = verified_email_check(&mut warnings, &None, fake_current_date);

        assert!(result.is_ok());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0], "You do not currently have a verified email address associated \
            with your crates.io account. Starting 2019-02-28, a verified email will be required to \
            publish crates. Visit https://crates.io/me to set and verify your email address.");
    }

    #[test]
    fn error_without_verified_email_after_2018_02_28() {
        let mut warnings = vec![];

        let fake_current_date = Utc.ymd(2019, 3, 1).and_hms(0, 0, 0);
        let result = verified_email_check(&mut warnings, &None, fake_current_date);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().description(),
            "A verified email address is required to \
             publish crates to crates.io. Visit https://crates.io/me to set and verify your email \
             address."
        );
    }
}
