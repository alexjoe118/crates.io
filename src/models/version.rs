use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::util::errors::{cargo_err, AppResult};

use crate::models::{Crate, Dependency, User};
use crate::schema::*;

// Queryable has a custom implementation below
#[derive(Clone, Identifiable, Associations, Debug, Queryable, Deserialize, Serialize)]
#[belongs_to(Crate)]
pub struct Version {
    pub id: i32,
    pub crate_id: i32,
    pub num: semver::Version,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub downloads: i32,
    pub features: serde_json::Value,
    pub yanked: bool,
    pub license: Option<String>,
    pub crate_size: Option<i32>,
    pub published_by: Option<i32>,
}

#[derive(Insertable, Debug)]
#[table_name = "versions"]
pub struct NewVersion {
    crate_id: i32,
    num: String,
    features: serde_json::Value,
    license: Option<String>,
    crate_size: Option<i32>,
    published_by: i32,
}

/// The highest version (semver order) and the most recently updated version.
/// Typically used for a single crate.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TopVersions {
    /// The "highest" version in terms of semver
    pub highest: Option<semver::Version>,
    /// The "highest" non-prerelease version
    pub highest_stable: Option<semver::Version>,
    /// The "newest" version in terms of publishing date
    pub newest: Option<semver::Version>,
}

impl TopVersions {
    /// Return both the newest (most recently updated) and the
    /// highest version (in semver order) for a list of `Version` instances.
    pub fn from_versions(versions: Vec<Version>) -> Self {
        Self::from_date_version_pairs(versions.into_iter().map(|v| (v.created_at, v.num)))
    }

    /// Return both the newest (most recently updated) and the
    /// highest version (in semver order) for a collection of date/version pairs.
    pub fn from_date_version_pairs<T>(pairs: T) -> Self
    where
        T: Clone + IntoIterator<Item = (NaiveDateTime, semver::Version)>,
    {
        let newest = pairs.clone().into_iter().max().map(|(_, v)| v);
        let highest = pairs.clone().into_iter().map(|(_, v)| v).max();

        let highest_stable = pairs
            .into_iter()
            .map(|(_, v)| v)
            .filter(|v| !v.is_prerelease())
            .max();

        Self {
            newest,
            highest,
            highest_stable,
        }
    }
}

impl Version {
    /// Returns (dependency, crate dependency name)
    pub fn dependencies(&self, conn: &PgConnection) -> QueryResult<Vec<(Dependency, String)>> {
        Dependency::belonging_to(self)
            .inner_join(crates::table)
            .select((dependencies::all_columns, crates::name))
            .order((dependencies::optional, crates::name))
            .load(conn)
    }

    pub fn record_readme_rendering(version_id_: i32, conn: &PgConnection) -> QueryResult<usize> {
        use crate::schema::readme_renderings::dsl::*;
        use diesel::dsl::now;

        diesel::insert_into(readme_renderings)
            .values(version_id.eq(version_id_))
            .on_conflict(version_id)
            .do_update()
            .set(rendered_at.eq(now))
            .execute(conn)
    }

    /// Gets the User who ran `cargo publish` for this version, if recorded.
    /// Not for use when you have a group of versions you need the publishers for.
    pub fn published_by(&self, conn: &PgConnection) -> Option<User> {
        match self.published_by {
            Some(pb) => users::table.find(pb).first(conn).ok(),
            None => None,
        }
    }
}

impl NewVersion {
    pub fn new(
        crate_id: i32,
        num: &semver::Version,
        features: &HashMap<String, Vec<String>>,
        license: Option<String>,
        license_file: Option<&str>,
        crate_size: i32,
        published_by: i32,
    ) -> AppResult<Self> {
        let features = serde_json::to_value(features)?;

        let mut new_version = NewVersion {
            crate_id,
            num: num.to_string(),
            features,
            license,
            crate_size: Some(crate_size),
            published_by,
        };

        new_version.validate_license(license_file)?;

        Ok(new_version)
    }

    pub fn save(
        &self,
        conn: &PgConnection,
        authors: &[String],
        published_by_email: &str,
    ) -> AppResult<Version> {
        use crate::schema::version_authors::{name, version_id};
        use crate::schema::versions::dsl::*;
        use diesel::dsl::exists;
        use diesel::{insert_into, select};

        conn.transaction(|| {
            let already_uploaded = versions
                .filter(crate_id.eq(self.crate_id))
                .filter(num.eq(&self.num));
            if select(exists(already_uploaded)).get_result(conn)? {
                return Err(cargo_err(&format_args!(
                    "crate version `{}` is already \
                     uploaded",
                    self.num
                )));
            }

            let version: Version = insert_into(versions).values(self).get_result(conn)?;

            insert_into(versions_published_by::table)
                .values((
                    versions_published_by::version_id.eq(version.id),
                    versions_published_by::email.eq(published_by_email),
                ))
                .execute(conn)?;

            let new_authors = authors
                .iter()
                .map(|s| (version_id.eq(version.id), name.eq(s)))
                .collect::<Vec<_>>();

            insert_into(version_authors::table)
                .values(&new_authors)
                .execute(conn)?;
            Ok(version)
        })
    }

    fn validate_license(&mut self, license_file: Option<&str>) -> AppResult<()> {
        if let Some(ref license) = self.license {
            for part in license.split('/') {
                license_exprs::validate_license_expr(part).map_err(|e| {
                    cargo_err(&format_args!(
                        "{}; see http://opensource.org/licenses \
                         for options, and http://spdx.org/licenses/ \
                         for their identifiers",
                        e
                    ))
                })?;
            }
        } else if license_file.is_some() {
            // If no license is given, but a license file is given, flag this
            // crate as having a nonstandard license. Note that we don't
            // actually do anything else with license_file currently.
            self.license = Some(String::from("non-standard"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TopVersions;
    use chrono::NaiveDateTime;

    #[track_caller]
    fn date(str: &str) -> NaiveDateTime {
        str.parse().unwrap()
    }

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    #[test]
    fn top_versions_empty() {
        let versions = vec![];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: None,
                highest_stable: None,
                newest: None,
            }
        );
    }

    #[test]
    fn top_versions_single() {
        let versions = vec![(date("2020-12-03T12:34:56"), version("1.0.0"))];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: Some(version("1.0.0")),
                highest_stable: Some(version("1.0.0")),
                newest: Some(version("1.0.0")),
            }
        );
    }

    #[test]
    fn top_versions_prerelease() {
        let versions = vec![(date("2020-12-03T12:34:56"), version("1.0.0-beta.5"))];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: Some(version("1.0.0-beta.5")),
                highest_stable: None,
                newest: Some(version("1.0.0-beta.5")),
            }
        );
    }

    #[test]
    fn top_versions_multiple() {
        let versions = vec![
            (date("2018-12-03T12:34:56"), version("1.0.0")),
            (date("2019-12-03T12:34:56"), version("2.0.0-alpha.1")),
            (date("2020-12-03T12:34:56"), version("1.1.0")),
            (date("2020-12-31T12:34:56"), version("1.0.4")),
        ];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: Some(version("2.0.0-alpha.1")),
                highest_stable: Some(version("1.1.0")),
                newest: Some(version("1.0.4")),
            }
        );
    }
}
