<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithoutChildrenOrChild<TabsPrimitive.ContentProps> & {
    children?: Snippet;
  } = $props();
</script>

<TabsPrimitive.Content
  bind:ref
  data-slot="tabs-content"
  class={cn(
    'focus-visible:border-ring focus-visible:ring-ring/50 mt-4 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-offset-2 focus-visible:ring-offset-background',
    className
  )}
  {...restProps}
>
  {@render children?.()}
</TabsPrimitive.Content>
