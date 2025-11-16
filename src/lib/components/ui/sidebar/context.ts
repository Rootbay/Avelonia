import { getContext, setContext } from 'svelte';
import { writable, derived, type Readable, type Writable } from 'svelte/store';
import { IsMobile } from '$lib/hooks/is-mobile';
import { SIDEBAR_KEYBOARD_SHORTCUT } from './constants';

export type SidebarStateProps = {
  open: () => boolean;
  setOpen: (open: boolean) => void;
};

class SidebarState {
  readonly props: SidebarStateProps;
  readonly open: Writable<boolean>;
  readonly openMobile: Writable<boolean>;
  readonly state: Readable<'expanded' | 'collapsed'>;
  setOpen: SidebarStateProps['setOpen'];
  #isMobile: IsMobile;

  constructor(props: SidebarStateProps) {
    this.props = props;
    this.setOpen = (value: boolean) => {
      props.setOpen(value);
      this.open.set(value);
    };
    this.#isMobile = new IsMobile();
    this.open = writable(this.props.open());
    this.openMobile = writable(false);
    this.state = derived(this.open, ($open) => ($open ? 'expanded' : 'collapsed'));
  }

  get isMobile() {
    return this.#isMobile.current;
  }

  handleShortcutKeydown = (e: KeyboardEvent) => {
    if (e.key === SIDEBAR_KEYBOARD_SHORTCUT && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      this.toggle();
    }
  };

  setOpenMobile = (value: boolean) => {
    this.openMobile.set(value);
  };

  syncOpen = (value: boolean) => {
    this.open.set(value);
  };

  toggle = () => {
    if (this.#isMobile.current) {
      this.openMobile.update((v) => !v);
    } else {
      this.setOpen(!this.props.open());
    }
  };
}

const SYMBOL_KEY = 'scn-sidebar';

export function setSidebar(props: SidebarStateProps): SidebarState {
  const sidebar = new SidebarState(props);
  setContext(Symbol.for(SYMBOL_KEY), sidebar);
  return sidebar;
}

export function useSidebar(): SidebarState {
  return getContext(Symbol.for(SYMBOL_KEY));
}
