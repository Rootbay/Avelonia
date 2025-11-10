<script lang="ts">
  import { Progress as ProgressPrimitive } from 'bits-ui';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    max = 100,
    value,
    indeterminate = false,
    ...restProps
  }: WithoutChildrenOrChild<ProgressPrimitive.RootProps> & { indeterminate?: boolean } = $props();
</script>

<ProgressPrimitive.Root
  bind:ref
  data-slot="progress"
  class={cn('bg-primary/20 relative h-2 w-full overflow-hidden rounded-full', className)}
  {value}
  {max}
  {...restProps}
>
  {#if indeterminate || value == null}
    <div class="absolute inset-0 overflow-hidden">
      <div
        class="h-full w-1/3 rounded-full bg-primary/70 will-change-transform animate-progress-indeterminate"
      ></div>
    </div>
  {:else}
    <div
      data-slot="progress-indicator"
      class="bg-primary h-full w-full flex-1 will-change-transform transition-transform duration-300 ease-out"
      style="transform: translateX(-{100 - (100 * (value ?? 0)) / (max ?? 1)}%)"
    ></div>
  {/if}
</ProgressPrimitive.Root>

<style>
  @keyframes progress-indeterminate {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(300%);
    }
  }
  .animate-progress-indeterminate {
    animation: progress-indeterminate 1.2s linear infinite;
  }
</style>
