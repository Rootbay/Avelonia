<script lang="ts">
  import { cn } from '$lib/utils';

  let { ref = $bindable(null), class: className, children, ...restProps } = $props();
</script>

<div
  bind:this={ref}
  data-slot="dropdown-menu-label"
  class={cn('px-2 py-1.5 text-sm font-semibold', className)}
  {...restProps}
>
  {@render children?.()}
</div>
