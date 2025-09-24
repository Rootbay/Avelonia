<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithoutChildrenOrChild<TabsPrimitive.RootProps> & {
    children?: Snippet;
  } = $props();
</script>

<TabsPrimitive.Root bind:ref data-slot="tabs" class={cn('w-full', className)} {...restProps}>
  {@render children?.()}
</TabsPrimitive.Root>
