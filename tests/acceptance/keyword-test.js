import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';

module('Acceptance | keywords', function(hooks) {
    setupApplicationTest(hooks);

    test('keyword/:keyword_id is accessible', async function(assert) {
        assert.expect(0);

        server.create('keyword', { id: 'network', keyword: 'network', crates_cnt: 38 });

        await visit('keywords/network');
        await a11yAudit(axeConfig);
    });

    test('keyword/:keyword_id index default sort is recent-downloads', async function(assert) {
        server.create('keyword', { id: 'network', keyword: 'network', crates_cnt: 38 });

        await visit('/keywords/network');

        assert.dom('[data-test-keyword-sort] [data-test-current-order]').hasText('Recent Downloads');
    });
});
