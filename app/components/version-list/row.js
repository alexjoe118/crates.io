import { computed } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class VersionRow extends Component {
  @service session;

  get releaseTrackTitle() {
    let { version } = this.args;
    if (version.yanked) {
      return 'This version was yanked';
    }
    if (version.isFirst) {
      return 'This is the first version that was released';
    }

    let { releaseTrack } = version;

    let modifiers = [];
    if (version.isPrerelease) {
      modifiers.push('prerelease');
    }
    if (version.isHighestOfReleaseTrack) {
      modifiers.push('latest');
    }

    let title = `Release Track: ${releaseTrack}`;
    if (modifiers.length !== 0) {
      title += ` (${modifiers.join(', ')})`;
    }
    return title;
  }

  @computed('args.version.crate.owner_user', 'session.currentUser.id')
  get isOwner() {
    return this.args.version.crate?.owner_user?.findBy('id', this.session.currentUser?.id);
  }
}
