import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | Categories', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/category_slugs', function () {
    test('empty case', async function (assert) {
      let response = await fetch('/api/v1/category_slugs');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        category_slugs: [],
      });
    });

    test('returns a category slugs list', async function (assert) {
      this.server.create('category', {
        category: 'no-std',
        description: 'Crates that are able to function without the Rust standard library.',
      });
      this.server.createList('category', 2);

      let response = await fetch('/api/v1/category_slugs');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        category_slugs: [
          {
            description: 'This is the description for the category called "Category 1"',
            id: 'category-1',
            slug: 'category-1',
          },
          {
            description: 'This is the description for the category called "Category 2"',
            id: 'category-2',
            slug: 'category-2',
          },
          {
            description: 'Crates that are able to function without the Rust standard library.',
            id: 'no-std',
            slug: 'no-std',
          },
        ],
      });
    });

    test('has no pagination', async function (assert) {
      this.server.createList('category', 25);

      let response = await fetch('/api/v1/category_slugs');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.equal(responsePayload.category_slugs.length, 25);
    });
  });
});
