<script lang="ts">
  import { icons } from '$lib/icons';
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { initDownloadListener, disposeDownloadListener } from '$lib/downloadManager';
  import { downloads } from '$lib/downloads';
  import type { Snippet } from 'svelte';

  import {
    Sidebar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupContent,
    SidebarMenu,
    SidebarMenuItem,
    SidebarMenuButton,
    SidebarProvider,
  } from '$lib/components/ui/sidebar';
  import { Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/ui/tooltip';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import { Toaster } from '$lib/components/ui/sonner';

  import '../app.css';

  onMount(() => {
    initDownloadListener();
  });
  onDestroy(() => {
    disposeDownloadListener();
  });

  let { children }: { children?: Snippet } = $props();
  let open = $state(true);
  const collapsed = $derived(!open);

  const activeCount = $derived(
    $downloads.filter(
      (d) => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued'
    ).length
  );

  function isActive(path: string) {
    return $page.url.pathname === path;
  }
</script>

<SidebarProvider bind:open class="h-screen w-full">
  <Sidebar class={collapsed ? 'w-16' : 'w-64'} aria-expanded={open}>
    <SidebarContent>
      <SidebarGroup>
        <SidebarGroupContent>
          <div class="flex items-center justify-between px-3 py-3">
            <div class="flex items-center gap-3">
              <img
                src="/favicon.png"
                alt="Avelonia Logo"
                class="h-10 w-10 rounded-full"
                width="40"
                height="40"
              />
              {#if !collapsed}
                <p class="font-medium text-base">Avelonia</p>
              {/if}
            </div>

            <Button
              variant="ghost"
              size="icon"
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
              onclick={() => (open = !open)}
            >
              <span aria-hidden="true">{@html icons.Dashboard}</span>
            </Button>
          </div>
        </SidebarGroupContent>
      </SidebarGroup>

      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton>
                {#if collapsed}
                  <Tooltip>
                    <TooltipTrigger>
                      <a
                        href="/dashboard"
                        aria-current={isActive('/dashboard') ? 'page' : undefined}
                        class="flex items-center gap-3"
                      >
                        <span class="size-6">{@html icons.Dashboard}</span>
                      </a>
                    </TooltipTrigger>
                    <TooltipContent>Dashboard</TooltipContent>
                  </Tooltip>
                {:else}
                  <a
                    href="/dashboard"
                    aria-current={isActive('/dashboard') ? 'page' : undefined}
                    class="flex items-center gap-3"
                  >
                    <span class="size-6">{@html icons.Dashboard}</span>
                    <span>Dashboard</span>
                  </a>
                {/if}
              </SidebarMenuButton>
            </SidebarMenuItem>

            <SidebarMenuItem>
              <SidebarMenuButton>
                {#if collapsed}
                  <Tooltip>
                    <TooltipTrigger>
                      <a
                        href="/optimize"
                        aria-current={isActive('/optimize') ? 'page' : undefined}
                        class="flex items-center gap-3"
                      >
                        <span class="size-6">{@html icons.Optimize}</span>
                      </a>
                    </TooltipTrigger>
                    <TooltipContent>Optimize</TooltipContent>
                  </Tooltip>
                {:else}
                  <a
                    href="/optimize"
                    aria-current={isActive('/optimize') ? 'page' : undefined}
                    class="flex items-center gap-3"
                  >
                    <span class="size-6">{@html icons.Optimize}</span>
                    <span>Optimize</span>
                  </a>
                {/if}
              </SidebarMenuButton>
            </SidebarMenuItem>

            <SidebarMenuItem>
              <SidebarMenuButton>
                {#if collapsed}
                  <Tooltip>
                    <TooltipTrigger>
                      <a
                        href="/downloader"
                        aria-current={isActive('/downloader') ? 'page' : undefined}
                        class="flex items-center gap-3"
                      >
                        <span class="size-6">{@html icons.Downloader}</span>
                        {#if activeCount > 0}
                          <Badge variant="secondary" aria-label={`Active downloads: ${activeCount}`}
                            >{activeCount}</Badge
                          >
                        {/if}
                      </a>
                    </TooltipTrigger>
                    <TooltipContent>Downloader</TooltipContent>
                  </Tooltip>
                {:else}
                  <a
                    href="/downloader"
                    aria-current={isActive('/downloader') ? 'page' : undefined}
                    class="flex items-center gap-3"
                  >
                    <span class="size-6">{@html icons.Downloader}</span>
                    <span>Downloader</span>
                    {#if activeCount > 0}
                      <Badge
                        variant="secondary"
                        class="ml-auto"
                        aria-label={`Active downloads: ${activeCount}`}>{activeCount}</Badge
                      >
                    {/if}
                  </a>
                {/if}
              </SidebarMenuButton>
            </SidebarMenuItem>

            <SidebarMenuItem>
              <SidebarMenuButton>
                {#if collapsed}
                  <Tooltip>
                    <TooltipTrigger>
                      <a
                        href="/cleaner"
                        aria-current={isActive('/cleaner') ? 'page' : undefined}
                        class="flex items-center gap-3"
                      >
                        <span class="size-6">{@html icons.Cleaner}</span>
                      </a>
                    </TooltipTrigger>
                    <TooltipContent>Cleaner</TooltipContent>
                  </Tooltip>
                {:else}
                  <a
                    href="/cleaner"
                    aria-current={isActive('/cleaner') ? 'page' : undefined}
                    class="flex items-center gap-3"
                  >
                    <span class="size-6">{@html icons.Cleaner}</span>
                    <span>Cleaner</span>
                  </a>
                {/if}
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
  </Sidebar>

  <main class="flex-1 overflow-y-auto p-8">
    {@render children?.()}
  </main>
</SidebarProvider>

<Toaster richColors closeButton duration={4000} position="bottom-right" />
