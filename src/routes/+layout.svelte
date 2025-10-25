<script lang="ts">
  import { page } from '$app/stores';
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
    Sun as SunIcon,
    Moon as MoonIcon,
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
  import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
    DialogClose,
    DialogTrigger,
  } from '$lib/components/ui/dialog';
  import { Tabs, TabsList, TabsTrigger, TabsContent } from '$lib/components/ui/tabs';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { startDownloadIntegrityWatch, stopDownloadIntegrityWatch } from '$lib/downloadIntegrity';

  import '../app.css';

  onMount(() => {
    initDownloadListener();
    startDownloadIntegrityWatch(20000);
  });
  onDestroy(() => {
    disposeDownloadListener();
    stopDownloadIntegrityWatch();
  });

  let { children }: { children?: Snippet } = $props();
  let open = $state(true);
  const collapsed = $derived(!open);

  type MenuIcon = Component<IconProps>;

  type ButtonSnippetContext = {
    props?: Record<string, unknown> & { class?: string };
  };

  const menuItems: Array<{
    href: string;
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
      (d) => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued'
    ).length
  );

  function isActive(path: string) {
    // Deprecated: avoid using store inside a function to preserve reactivity
    // Kept for backward compatibility if used elsewhere
    return $page.url.pathname === path;
  }

  type PagePreference = {
    showInSidebar: boolean;
    enableNotifications: boolean;
  };

  const settingsTabItems = menuItems.map((item) => ({
    value: item.href.slice(1) || 'root',
    label: item.label,
    description: `Adjust preferences for the ${item.label} page.`,
  }));

  let settingsOpen = $state(false);
  let activeSettingsTab = $state(settingsTabItems[0]?.value ?? 'dashboard');
  let pagePreferences = $state<Record<string, PagePreference>>(
    Object.fromEntries(
      settingsTabItems.map((tab) => [
        tab.value,
        {
          showInSidebar: true,
          enableNotifications: tab.value === 'downloader',
        },
      ])
    ) as Record<string, PagePreference>
  );
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
        class="absolute top-1/2 -translate-y-1/2 right-0 z-20 opacity-0 pointer-events-none group-hover/sidebar:opacity-100 group-hover/sidebar:pointer-events-auto h-8 w-8 flex items-center justify-center rounded-l-md rounded-r-none border bg-background text-foreground shadow-xs hover:bg-accent hover:text-accent-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring/50 transition"
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
            {#each menuItems as item}
              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={$page.url.pathname === item.href}
                  tooltipContent={collapsed ? item.label : undefined}
                >
                  {#snippet child({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  {@const Icon = item.icon}
                  <a
                    {...restProps}
                    href={item.href}
                    data-sveltekit-preload-data={item.href === '/optimize' ? 'off' : undefined}
                    data-sveltekit-preload-code={item.href === '/optimize' ? 'off' : undefined}
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
                          ? 'text-[var(--sidebar-accent-foreground)]'
                          : 'text-current'
                      )}
                      style={$page.url.pathname === item.href ? 'stroke-width:1.5' : 'stroke-width:1'}
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
          <Dialog bind:open={settingsOpen}>
            <div class="px-3 py-3">
              {#if collapsed}
                <div class="flex flex-col items-center gap-2">
                  {#snippet ThemeToggleTrigger({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
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
                    <SunIcon class="h-[1.2rem] w-[1.2rem] text-current rotate-0 scale-100 !transition-all dark:-rotate-90 dark:scale-0" />
                    <MoonIcon class="absolute h-[1.2rem] w-[1.2rem] text-current rotate-90 scale-0 !transition-all dark:rotate-0 dark:scale-100" />
                  </Button>
                  {/snippet}
                  <Tooltip>
                    <TooltipTrigger child={ThemeToggleTrigger} />
                    <TooltipContent side="right" align="center">Toggle theme</TooltipContent>
                  </Tooltip>

                  {#snippet SettingsButtonContent({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  <Button
                    {...restProps}
                    variant="outline"
                    size="icon"
                    class={cn('relative', propsClass)}
                    aria-label="Open settings"
                  >
                    <SettingsIcon class="h-[1.2rem] w-[1.2rem] text-current" />
                  </Button>
                  {/snippet}
                  {#snippet SettingsTrigger({ props }: ButtonSnippetContext)}
                  <DialogTrigger {...((props ?? {}) as Record<string, unknown>)} child={SettingsButtonContent} />
                  {/snippet}
                  <Tooltip>
                    <TooltipTrigger child={SettingsTrigger} />
                    <TooltipContent side="right" align="center">Settings</TooltipContent>
                  </Tooltip>
                </div>
              {:else}
                <div class="flex items-center gap-3">
                  {#snippet SettingsButtonExpanded({ props }: ButtonSnippetContext)}
                  {@const rawProps = (props ?? {}) as Record<string, unknown> & { class?: string }}
                  {@const { class: propsClass, ...restWithoutClass } = rawProps}
                  {@const restProps = restWithoutClass as Record<string, unknown>}
                  <Button
                    {...restProps}
                    variant="outline"
                    size="sm"
                    class={cn('flex items-center gap-2', propsClass)}
                    aria-label="Open settings"
                  >
                    <SettingsIcon class="h-[1.2rem] w-[1.2rem] text-current" />
                    <span>Settings</span>
                  </Button>
                  {/snippet}
                  <DialogTrigger child={SettingsButtonExpanded} />
                  <Button
                    onclick={toggleMode}
                    variant="outline"
                    size="icon"
                    class="relative ml-auto"
                    aria-label="Toggle theme"
                  >
                    <SunIcon
                      class="h-[1.2rem] w-[1.2rem] text-current rotate-0 scale-100 !transition-all dark:-rotate-90 dark:scale-0"
                    />
                    <MoonIcon
                      class="absolute h-[1.2rem] w-[1.2rem] text-current rotate-90 scale-0 !transition-all dark:rotate-0 dark:scale-100"
                    />
                  </Button>
                </div>
              {/if}
            </div>

            <DialogContent class="sm:max-w-xl">
              <DialogHeader>
                <DialogTitle>Settings</DialogTitle>
                <DialogDescription>Adjust preferences for each page.</DialogDescription>
              </DialogHeader>

              <Tabs value={activeSettingsTab} onValueChange={(value) => (activeSettingsTab = value)} class="flex flex-col gap-4">
                <TabsList class="flex flex-wrap gap-1">
                  {#each settingsTabItems as tab}
                    <TabsTrigger value={tab.value}>{tab.label}</TabsTrigger>
                  {/each}
                </TabsList>

                {#each settingsTabItems as tab}
                  <TabsContent value={tab.value} class="space-y-4">
                    <p class="text-sm text-muted-foreground">{tab.description}</p>
                    <div class="space-y-3">
                      <div
                        class="flex items-start gap-3 rounded-md border border-border/60 bg-muted/20 p-3"
                      >
                        <Checkbox
                          id={`${tab.value}-sidebar`}
                          bind:checked={pagePreferences[tab.value].showInSidebar}
                        />
                        <div class="space-y-1">
                          <p class="text-sm font-medium">Show in sidebar</p>
                          <p class="text-xs text-muted-foreground">
                            Keep the {tab.label} page accessible from the navigation.
                          </p>
                        </div>
                      </div>
                      <div
                        class="flex items-start gap-3 rounded-md border border-border/60 bg-muted/20 p-3"
                      >
                        <Checkbox
                          id={`${tab.value}-notifications`}
                          bind:checked={pagePreferences[tab.value].enableNotifications}
                        />
                        <div class="space-y-1">
                          <p class="text-sm font-medium">Enable notifications</p>
                          <p class="text-xs text-muted-foreground">
                            Receive reminders for {tab.label.toLowerCase()} updates.
                          </p>
                        </div>
                      </div>
                    </div>
                  </TabsContent>
                {/each}
              </Tabs>

              <DialogFooter>
                <DialogClose>
                  <Button variant="ghost">Cancel</Button>
                </DialogClose>
                <DialogClose>
                  <Button>Done</Button>
                </DialogClose>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
  </Sidebar>

  <main class="flex-1 overflow-y-auto p-8">
    {@render children?.()}
  </main>
</SidebarProvider>

<Toaster richColors closeButton duration={4000} position="bottom-right" />
