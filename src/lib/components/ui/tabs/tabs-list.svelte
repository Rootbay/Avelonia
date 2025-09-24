<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithoutChildrenOrChild<TabsPrimitive.ListProps> & {
    children?: Snippet;
  } = $props();
</script>

<TabsPrimitive.List
  bind:ref
  data-slot="tabs-list"
  class={cn(
    'inline-flex h-9 items-center justify-center rounded-lg bg-muted p-1 text-muted-foreground',
    className
  )}
  {...restProps}
>
  {@render children?.()}
</TabsPrimitive.List>
