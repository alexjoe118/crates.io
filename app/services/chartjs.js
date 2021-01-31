import { assert } from '@ember/debug';
import Service from '@ember/service';

import addDays from 'date-fns/addDays';
import differenceInDays from 'date-fns/differenceInDays';
import endOfDay from 'date-fns/endOfDay';
import format from 'date-fns/format';
import startOfDay from 'date-fns/startOfDay';
import { task } from 'ember-concurrency';

export default class ChartJsLoader extends Service {
  @(task(function* () {
    let Chart = yield import('chart.js').then(module => module.default);

    Chart._adapters._date.override({
      _id: 'date-fns', // DEBUG

      formats() {
        return { day: 'MMM d' };
      },

      parse(value) {
        if (value === null || value === undefined) {
          return null;
        }
        assert('`value` must be a `Date`', value instanceof Date);
        return !isNaN(value) ? value.getTime() : null;
      },

      format(time, fmt) {
        return format(time, fmt, this.options);
      },

      add(time, amount, unit) {
        assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
        return addDays(time, amount);
      },

      diff(max, min, unit) {
        assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
        return differenceInDays(max, min);
      },

      startOf(time, unit) {
        assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
        return startOfDay(time);
      },

      endOf(time, unit) {
        assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
        return endOfDay(time);
      },
    });

    Chart.platform.disableCSSInjection = true;
    return Chart;
  }).drop())
  loadTask;
}
