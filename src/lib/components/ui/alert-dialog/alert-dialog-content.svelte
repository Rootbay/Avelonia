<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
  import AlertDialogOverlay from './alert-dialog-overlay.svelte';
  import XIcon from '@lucide/svelte/icons/x';
  import { cn, type WithoutChildrenOrChild, type WithElementRef } from '$lib/utils.js';
  import type { Snippet } from 'svelte';

  let {
    ref = $bindable(null),
    class: className,
    portalProps,
    children,
    showCloseButton = true,
    ...restProps
  }: WithElementRef<AlertDialogPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<AlertDialogPrimitive.PortalProps>;
    children?: Snippet;
    showCloseButton?: boolean;
  } = $props();
</script>

<AlertDialogPrimitive.Portal {...portalProps}>
  <AlertDialogOverlay />
  <AlertDialogPrimitive.Content
    bind:ref
    data-slot="alert-dialog-content"
    class={cn(
      'bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed left-1/2 top-1/2 z-50 grid translate-x-[-50%] translate-y-[-50%] gap-4 rounded-lg border p-6 shadow-lg duration-200',
      className,
      'w-[min(90svw,40rem)] max-w-[90svw] max-h-[80svh] overflow-auto'
    )}
    {...restProps}
  >
    {@render children?.()}
    {#if showCloseButton}
      <AlertDialogPrimitive.Cancel
        class="ring-offset-background focus:ring-ring rounded-xs focus:outline-hidden absolute end-6 top-6 opacity-70 transition-opacity hover:opacity-100 focus:ring-2 focus:ring-offset-2 disabled:pointer-events-none"
        aria-label="Close"
        title="Close"
      >
        <XIcon class="size-4" />
        <span class="sr-only">Close</span>
      </AlertDialogPrimitive.Cancel>
    {/if}
  </AlertDialogPrimitive.Content>
</AlertDialogPrimitive.Portal>
