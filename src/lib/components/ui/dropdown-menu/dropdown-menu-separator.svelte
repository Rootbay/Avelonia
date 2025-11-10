<script lang="ts">
  import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils';

  let { class: className, ...restProps }: DropdownMenuPrimitive.SeparatorProps = $props();
</script>

<DropdownMenuPrimitive.Separator
  data-slot="dropdown-menu-separator"
  class={cn('mx-1 my-1 h-px bg-border', className)}
  {...restProps}
/>
