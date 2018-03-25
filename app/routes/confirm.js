import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
    flashMessages: service(),
    ajax: service(),
    session: service(),

    async model(params) {
        try {
            await this.get('ajax').raw(`/api/v1/confirm/${params.email_token}`, { method: 'PUT', data: {} });

            /*  We need this block to reload the user model from the database,
                without which if we haven't submitted another GET /me after
                clicking the link and before checking their account info page,
                the user will still see that their email has not yet been
                validated and could potentially be confused, resend the email,
                and set up a situation where their email has been verified but
                they have an unverified token sitting in the DB.

                Suggestions of a more ideomatic way to fix/test this are welcome!
            */
            if (this.get('session.isLoggedIn')) {
                this.get('ajax').request('/api/v1/me').then((response) => {
                    this.get('session').set('currentUser', this.store.push(this.store.normalize('user', response.user)));
                });
            }

        } catch(error) {
            if (error.payload) {
                this.get('flashMessages').queue(`Error in email confirmation: ${error.payload.errors[0].detail}`);
                return this.replaceWith('index');
            } else {
                this.get('flashMessages').queue(`Unknown error in email confirmation`);
                return this.replaceWith('index');
            }
        }
    }
});
