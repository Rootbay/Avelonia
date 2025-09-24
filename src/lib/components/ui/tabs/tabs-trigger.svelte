<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithoutChildrenOrChild<TabsPrimitive.TriggerProps> & {
    children?: Snippet;
  } = $props();
</script>

<TabsPrimitive.Trigger
  bind:ref
  data-slot="tabs-trigger"
  class={cn(
    'focus-visible:border-ring focus-visible:ring-ring/50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50',
    className
  )}
  {...restProps}
>
  {@render children?.()}
</TabsPrimitive.Trigger>
