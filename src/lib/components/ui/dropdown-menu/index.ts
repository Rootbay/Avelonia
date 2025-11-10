import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';

import Trigger from './dropdown-menu-trigger.svelte';
import Content from './dropdown-menu-content.svelte';
import Item from './dropdown-menu-item.svelte';
import Label from './dropdown-menu-label.svelte';
import Separator from './dropdown-menu-separator.svelte';

const Root = DropdownMenuPrimitive.Root;
const Portal = DropdownMenuPrimitive.Portal;
const Group = DropdownMenuPrimitive.Group;

export {
  Root,
  Portal,
  Group,
  Trigger,
  Content,
  Item,
  Label,
  Separator,
  Root as DropdownMenu,
  Portal as DropdownMenuPortal,
  Group as DropdownMenuGroup,
  Trigger as DropdownMenuTrigger,
  Content as DropdownMenuContent,
  Item as DropdownMenuItem,
  Label as DropdownMenuLabel,
  Separator as DropdownMenuSeparator,
};
