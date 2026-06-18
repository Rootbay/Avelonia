<script lang="ts">
  import { page } from '$app/stores';
  import { resolve } from '$app/paths';
  import { onMount, onDestroy } from 'svelte';
  import { initDownloadListener, disposeDownloadListener } from '$lib/downloadManager';
  import { downloads } from '$lib/downloads';
  import type { Component, Snippet } from 'svelte';
  import type { IconProps } from '@lucide/svelte';
  import {
    LayoutDashboard,
    Gauge,
    Download,
    Eraser,
    ChevronLeft,
    ChevronRight,
    Sun,
    Moon,
    Settings as SettingsIcon,
  } from '@lucide/svelte';
  import { ModeWatcher, toggleMode } from 'mode-watcher';
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
  import { cn } from '$lib/utils.js';
  import { startDownloadIntegrityWatch, stopDownloadIntegrityWatch } from '$lib/downloadIntegrity';
  import { startInstallPresenceWatch, stopInstallPresenceWatch } from '$lib/downloadManager';
  import SettingsDialog from '$lib/components/SettingsDialog.svelte';
  import VtScanDialog from '$lib/components/VtScanDialog.svelte';
  import { useVtScan } from '$lib/hooks/useVtScan.svelte';
  import { useCleanerScan } from '$lib/hooks/useCleanerScan.svelte';
  import '../app.css';

  let { children }: { children?: Snippet } = $props();
  let open = $state(true);
  const collapsed = $derived(!open);
  let scanDialogOpen = $state(false);
  let settingsOpen = $state(false);

  useVtScan();
  useCleanerScan();

  onMount(() => {
    initDownloadListener();
    startDownloadIntegrityWatch(20000);
    startInstallPresenceWatch(20000);
  });

  onDestroy(() => {
    disposeDownloadListener();
    stopDownloadIntegrityWatch();
    stopInstallPresenceWatch();
  });

  type MenuIcon = Component<IconProps>;
  type AppRoute = '/dashboard' | '/optimize' | '/downloader' | '/cleaner';

  const menuItems: Array<{
    href: AppRoute;
    label: string;
    icon: MenuIcon;
    showBadge?: boolean;
  }> = [
    { href: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { href: '/optimize', label: 'Optimize', icon: Gauge },
    { href: '/downloader', label: 'Downloader', icon: Download, showBadge: true },
    { href: '/cleaner', label: 'Cleaner', icon: Eraser },
  ];

  const activeCount = $derived(
    $downloads.filter(
      (d) =>
        d.status === 'downloading' ||
        d.status === 'pending' ||
        d.status === 'queued' ||
        d.status === 'verifying'
    ).length
  );

  type ButtonSnippetContext = {
    props?: Record<string, unknown> & { class?: string };
  };
</script>

<ModeWatcher />

<SidebarProvider bind:open style="--sidebar-width-icon: 60px;" class="h-screen w-full">
  <Sidebar
    collapsible="icon"
    class="group/sidebar relative transition-[width] duration-200"
    aria-expanded={open}
  >
    <SidebarContent class="relative overflow-x-hidden">
      <button
        type="button"
        class="absolute top-1/2 -translate-y-1/2 right-0 z-20 opacity-0 pointer-events-none group-hover/sidebar:opacity-100 group-hover/sidebar:pointer-events-auto h-8 w-8 flex items-center justify-center rounded-l-md rounded-r-none border bg-background text-foreground shadow-xs hover:bg-accent hover:text-accent-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring/50 transition cursor-pointer"
        aria-label={open ? 'Collapse sidebar' : 'Expand sidebar'}
        title={open ? 'Collapse sidebar' : 'Expand sidebar'}
        onclick={() => (open = !open)}
      >
        {#if open}
          <ChevronLeft class="size-5" />
        {:else}
          <ChevronRight class="size-5" />
        {/if}
      </button>
      <SidebarGroup>
        <SidebarGroupContent>
          {#if collapsed}
            <div class="flex justify-center px-3 py-3">
              <div class="relative flex h-10 w-10 items-center justify-center">
                <img
                  src="/favicon.png"
                  alt="Avelonia Logo"
                  class="h-10 w-10 rounded-full aspect-square object-contain"
                  width="25"
                  height="25"
                />
              </div>
            </div>
          {:else}
            <div class="flex items-center px-3 py-3">
              <div class="flex items-center gap-3">
                <img
                  src="/favicon.png"
                  alt="Avelonia Logo"
                  class="h-10 w-10 rounded-full aspect-square object-contain transition-transform duration-200"
                  width="25"
                  height="25"
                />
                <p class="font-medium text-base">Avelonia</p>
              </div>
            </div>
          {/if}
        </SidebarGroupContent>
      </SidebarGroup>

      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu>
            {#each menuItems as item (item.href)}
              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={$page.url.pathname === item.href}
                  tooltipContent={collapsed ? item.label : undefined}
                >
                  {#snippet child({ props }: ButtonSnippetContext)}
                    {@const rawProps = (props ?? {}) as Record<string, unknown> & {
                      class?: string;
                    }}
                    {@const { class: propsClass, ...restWithoutClass } = rawProps}
                    {@const restProps = restWithoutClass as Record<string, unknown>}
                    {@const Icon = item.icon}
                    <a
                      {...restProps}
                      href={resolve(item.href)}
                      data-sveltekit-preload-data={[
                        '/optimize',
                        '/downloader',
                        '/cleaner',
                      ].includes(item.href)
                        ? 'off'
                        : undefined}
                      data-sveltekit-preload-code={[
                        '/optimize',
                        '/downloader',
                        '/cleaner',
                      ].includes(item.href)
                        ? 'off'
                        : undefined}
                      aria-current={$page.url.pathname === item.href ? 'page' : undefined}
                      class={cn(
                        'flex items-center transition-colors',
                        collapsed ? 'justify-center relative mx-auto' : 'gap-3 w-full',
                        propsClass
                      )}
                    >
                      <Icon
                        class={cn(
                          'size-5 shrink-0',
                          collapsed && $page.url.pathname === item.href
                            ? 'text-sidebar-accent-foreground'
                            : 'text-current'
                        )}
                        style={$page.url.pathname === item.href
                          ? 'stroke-width:1.5'
                          : 'stroke-width:1'}
                        aria-hidden="true"
                      />
                      {#if !collapsed}
                        <span>{item.label}</span>
                      {/if}
                      {#if item.showBadge && activeCount > 0}
                        {#if collapsed}
                          <Badge
                            variant="secondary"
                            class="absolute -top-1 -right-1 h-4 min-w-4 px-1 text-[10px] leading-none flex items-center justify-center"
                            aria-label={`Active downloads: ${activeCount}`}
                          >
                            {activeCount}
                          </Badge>
                        {:else}
                          <Badge
                            variant="secondary"
                            class="ml-auto"
                            aria-label={`Active downloads: ${activeCount}`}
                          >
                            {activeCount}
                          </Badge>
                        {/if}
                      {/if}
                    </a>
                  {/snippet}
                </SidebarMenuButton>
              </SidebarMenuItem>
            {/each}
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>

      <SidebarGroup class="mt-auto pb-4">
        <SidebarGroupContent>
          <div class="px-3 py-3">
            {#if collapsed}
              <div class="flex flex-col items-center gap-2">
                {#snippet ThemeToggleTrigger({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & {
                    class?: string;
                  }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  <Button
                    {...restProps}
                    variant="outline"
                    size="icon"
                    class={cn('relative', propsClass)}
                    onclick={toggleMode}
                    aria-label="Toggle theme"
                  >
                    <Sun
                      class="h-[1.2rem] w-[1.2rem] text-current rotate-0 scale-100 transition-all! dark:-rotate-90 dark:scale-0"
                    />
                    <Moon
                      class="absolute h-[1.2rem] w-[1.2rem] text-current rotate-90 scale-0 transition-all! dark:rotate-0 dark:scale-100"
                    />
                  </Button>
                {/snippet}
                <Tooltip>
                  <TooltipTrigger child={ThemeToggleTrigger} />
                  <TooltipContent side="right" align="center">Toggle theme</TooltipContent>
                </Tooltip>

                {#snippet SettingsButtonContent({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & {
                    class?: string;
                  }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  <Button
                    {...restProps}
                    variant="outline"
                    size="icon"
                    class={cn('relative', propsClass)}
                    aria-label="Open settings"
                    onclick={() => (settingsOpen = true)}
                  >
                    <SettingsIcon class="h-[1.2rem] w-[1.2rem] text-current" />
                  </Button>
                {/snippet}
                <Tooltip>
                  <TooltipTrigger child={SettingsButtonContent} />
                  <TooltipContent side="right" align="center">Settings</TooltipContent>
                </Tooltip>
              </div>
            {:else}
              <div class="flex items-center gap-3">
                <Button
                  variant="outline"
                  size="sm"
                  class="flex items-center gap-2"
                  aria-label="Open settings"
                  onclick={() => (settingsOpen = true)}
                >
                  <SettingsIcon class="h-[1.2rem] w-[1.2rem] text-current" />
                  <span>Settings</span>
                </Button>
                <Button
                  onclick={toggleMode}
                  variant="outline"
                  size="icon"
                  class="relative ml-auto"
                  aria-label="Toggle theme"
                >
                  <Sun
                    class="h-[1.2rem] w-[1.2rem] text-current rotate-0 scale-100 transition-all! dark:-rotate-90 dark:scale-0"
                  />
                  <Moon
                    class="absolute h-[1.2rem] w-[1.2rem] text-current rotate-90 scale-0 transition-all! dark:rotate-0 dark:scale-100"
                  />
                </Button>
              </div>
            {/if}
          </div>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
  </Sidebar>

  <main class="flex-1 overflow-y-auto p-6">
    {@render children?.()}
  </main>
</SidebarProvider>

<SettingsDialog bind:open={settingsOpen} />
<VtScanDialog bind:open={scanDialogOpen} />

<Toaster richColors closeButton duration={4000} position="bottom-right" />
