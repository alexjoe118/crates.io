import Component from '@ember/component';
import { empty } from '@ember/object/computed';
import { computed } from '@ember/object';
import { inject as service } from '@ember/service';

export default Component.extend({
    ajax: service(),
    flashMessages: service(),

    type: '',
    value: '',
    isEditing: false,
    user: null,
    disableSave: empty('user.email'),
    notValidEmail: false,
    prevEmail: '',
    emailIsNull: computed('user.email', function() {
        let email = this.get('user.email');
        return (email == null);
    }),
    emailNotVerified: computed('user.email', 'user.email_verified', function() {
        let email = this.get('user.email');
        let verified = this.get('user.email_verified');

        if (email != null && !verified) {
            return true;
        } else {
            return false;
        }
    }),
    isError: false,
    emailError: '',

    actions: {
        editEmail() {
            let email = this.get('value');
            let isEmailNull = function(email) {
                return (email == null);
            };

            this.set('emailIsNull', isEmailNull(email));
            this.set('isEditing', true);
            this.set('prevEmail', this.get('value'));
        },

        saveEmail() {
            let userEmail = this.get('value');
            let user = this.get('user');

            let emailIsProperFormat = function(userEmail) {
                let regExp = /^\S+@\S+\.\S+$/;
                return regExp.test(userEmail);
            };

            if (!emailIsProperFormat(userEmail)) {
                this.set('notValidEmail', true);
                return;
            }

            user.set('email', userEmail);
            user.save()
                .then(() => this.set('serverError', null))
                .catch(err => {
                    let msg;
                    if (err.errors && err.errors[0] && err.errors[0].detail) {
                        msg = `An error occurred while saving this email, ${err.errors[0].detail}`;
                    } else {
                        msg = 'An unknown error occurred while saving this email.';
                    }
                    this.set('serverError', msg);
                    this.set('isError', true);
                    this.set('emailError', `Error in saving email: ${msg}`);
                });

            this.set('isEditing', false);
            this.set('notValidEmail', false);
        },

        cancelEdit() {
            this.set('isEditing', false);
            this.set('value', this.get('prevEmail'));
        },

        resendEmail() {
            let user = this.get('user');

            this.get('ajax').raw(`/api/v1/users/${user.id}/resend`, { method: 'PUT',
                user: {
                    avatar: user.avatar,
                    email: user.email,
                    email_verified: user.email_verified,
                    kind: user.kind,
                    login: user.login,
                    name: user.name,
                    url: user.url
                }
            }).catch((error) => {
                if (error.payload) {
                    this.set('isError', true);
                    this.set('emailError', `Error in resending message: ${error.payload.errors[0].detail}`);
                } else {
                    this.set('isError', true);
                    this.set('emailError', 'Unknown error in resending message');
                }
            });
        }
    }
});
