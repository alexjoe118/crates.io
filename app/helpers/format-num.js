import Ember from 'ember';

export function formatNum(value) {
    if (value === 0) { return "0"; }

    var ret = "";
    var cnt = 0;
    while (value > 0) {
        if (cnt > 0 && cnt % 3 === 0) {
            ret = "," + ret;
            cnt = 0;
        }
        ret = (value % 10) + ret;
        cnt += 1;
        value = Math.floor(value / 10);
    }
    return ret;
}

export default Ember.Helper.helper(params => formatNum(params[0]));
