import Ember from 'ember';

export default Ember.Service.extend({
    message: null,
    _nextMessage: null,

    show(message) {
        this.set('message', message);
    },

    queue(message) {
        this.set('_nextMessage', message);
    },

    step() {
        this.set('message', this.get('_nextMessage'));
        this.set('_nextMessage', null);
    }
});
