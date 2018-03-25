import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit } from 'ember-native-dom-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';

module('Acceptance | team page', function(hooks) {
    setupApplicationTest(hooks);

    test('is accessible', async function(assert) {
        assert.expect(0);

        server.loadFixtures();

        await visit('/teams/github:org:thehydroimpulse');
        await a11yAudit(axeConfig);
    });

    test('has team organization display', async function(assert) {
        server.loadFixtures();

        await visit('/teams/github:org:thehydroimpulse');

        assert.dom('[data-test-heading] [data-test-org-name]').hasText('org');
        assert.dom('[data-test-heading] [data-test-team-name]').hasText('thehydroimpulseteam');
    });

    test('has link to github in team header', async function(assert) {
        server.loadFixtures();

        await visit('/teams/github:org:thehydroimpulse');

        assert.dom('[data-test-heading] [data-test-github-link]')
            .hasAttribute('href', 'https://github.com/org_test');
    });

    test('github link has image in team header', async function(assert) {
        server.loadFixtures();

        await visit('/teams/github:org:thehydroimpulse');

        assert.dom('[data-test-heading] [data-test-github-link] img')
            .hasAttribute('src', '/assets/GitHub-Mark.svg');
    });

    test('team organization details has github profile icon', async function(assert) {
        server.loadFixtures();

        await visit('/teams/github:org:thehydroimpulse');

        assert.dom('[data-test-heading] [data-test-avatar]')
            .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
    });
});
