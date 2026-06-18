export type TweakItem = {
  id: string;
  label: string;
  description: string;
};

export const essentialTweaks: TweakItem[] = [
  {
    id: 'create_restore_point',
    label: 'Create Restore Point',
    description: 'Capture a system snapshot before applying any tweaks.',
  },
  {
    id: 'disable_consumer_features',
    label: 'Disable Consumer Features',
    description: 'Remove bundled consumer services and pop-ups that run in the background.',
  },
  {
    id: 'disable_telemetry',
    label: 'Disable Telemetry',
    description: 'Stop telemetry services from sending diagnostics data to Microsoft.',
  },
  {
    id: 'disable_activity_history',
    label: 'Disable Activity History',
    description: 'Keep Windows from building a timeline of your activities.',
  },
  {
    id: 'disable_explorer_discovery',
    label: 'Disable Explorer Automatic Folder Discovery',
    description: 'Prevent Explorer from auto-detecting folders and reconfiguring layouts.',
  },
  {
    id: 'disable_gamedvr',
    label: 'Disable GameDVR',
    description: 'Turn off Game Bar recording components that add CPU overhead.',
  },
  {
    id: 'disable_hibernation',
    label: 'Disable Hibernation',
    description: 'Remove the hibernation file to reclaim disk space unless the alternate is set.',
  },
  {
    id: 'disable_homegroup',
    label: 'Disable Homegroup',
    description: 'Stop the deprecated HomeGroup service from consuming resources.',
  },
  {
    id: 'disable_location_tracking',
    label: 'Disable Location Tracking',
    description: 'Prevent Windows and apps from tracking your location.',
  },
  {
    id: 'disable_storage_sense',
    label: 'Disable Storage Sense',
    description: 'Avoid automatic storage cleanups that may delete cached files you rely on.',
  },
  {
    id: 'disable_wifi_sense',
    label: 'Disable Wi-Fi Sense',
    description: 'Keep Windows from sharing wireless credentials with your contacts.',
  },
  {
    id: 'enable_end_task',
    label: 'Enable End Task With Right Click',
    description: 'Add an End Task shortcut in the right-click context menu for Task Manager.',
  },
  {
    id: 'run_disk_cleanup',
    label: 'Run Disk Cleanup',
    description: 'Trigger a clean sweep of temporary data, log files, and thumbnails.',
  },
  {
    id: 'terminal_powershell7_default',
    label: 'Change Windows Terminal Default to PowerShell 7',
    description: 'Point the default Windows Terminal profile to PowerShell 7 instead of 5.',
  },
  {
    id: 'disable_powershell7_telemetry',
    label: 'Disable PowerShell 7 Telemetry',
    description: 'Turn off telemetry baked into PowerShell 7 when it launches.',
  },
  {
    id: 'disable_recall',
    label: 'Disable Recall',
    description: 'Prevent Recall (Windows clipboard history) from capturing sensitive data.',
  },
  {
    id: 'set_hibernation_default',
    label: 'Set Hibernation as Default (Good for Laptops)',
    description: 'Prioritize hibernation behavior tailored for laptops.',
  },
  {
    id: 'services_manual',
    label: 'Set Services to Manual',
    description: 'Set select Windows services to manual so they start only when needed.',
  },
  {
    id: 'debloat_brave',
    label: 'Debloat Brave',
    description: 'Remove bundled Brave telemetry and background helpers.',
  },
  {
    id: 'debloat_edge',
    label: 'Debloat Edge',
    description: 'Strip Edge housekeeping processes that run even when the browser is idle.',
  },
  {
    id: 'enable_game_mode',
    label: 'Enable Windows Game Mode',
    description:
      'Prioritize your gaming experience by minimizing background activities and optimizing resource allocation.',
  },
];

export const advancedTweaks: TweakItem[] = [
  {
    id: 'block_adobe_network',
    label: 'Adobe Network Block',
    description: 'Prevent Adobe apps from calling home to telemetry and licensing servers.',
  },
  {
    id: 'debloat_adobe',
    label: 'Adobe Debloat',
    description: 'Disable unneeded Adobe helper services and auto-start components.',
  },
  {
    id: 'disable_ipv6',
    label: 'Disable IPv6',
    description: 'Turn off IPv6 if your environment relies solely on IPv4.',
  },
  {
    id: 'prefer_ipv4',
    label: 'Prefer IPv4 Over IPv6',
    description: 'Set Windows to prefer IPv4 when both stacks are available.',
  },
  {
    id: 'disable_teredo',
    label: 'Disable Teredo',
    description: 'Block the Teredo tunneling interface that is rarely needed anymore.',
  },
  {
    id: 'disable_background_apps',
    label: 'Disable Background Apps',
    description: 'Stop universal Windows apps from running in the background.',
  },
  {
    id: 'disable_fullscreen_optimizations',
    label: 'Disable Fullscreen Optimizations',
    description: 'Prevent Windows from modifying fullscreen behavior that can add input lag.',
  },
  {
    id: 'enable_hags',
    label: 'Enable HAGS',
    description:
      'Enable Hardware-Accelerated GPU Scheduling to reduce input lag and support frame generation.',
  },
  {
    id: 'disable_nagle',
    label: "Disable Nagle's Algorithm",
    description:
      'Disables TCP batching to improve Ethernet gaming responsiveness and reduce latency.',
  },
  {
    id: 'disable_network_throttling',
    label: 'Disable Network Throttling',
    description:
      'Prevent Windows from throttling non-multimedia network traffic when games are running.',
  },
  {
    id: 'optimize_system_responsiveness',
    label: 'Optimize System Responsiveness',
    description: 'Ensure games get 100% CPU priority over background services.',
  },
  {
    id: 'optimize_game_priority',
    label: 'Optimize Game Priority',
    description: 'Configure system gaming tasks to run with high GPU and CPU resource priority.',
  },
  {
    id: 'ethernet_low_latency',
    label: 'Ethernet Adapter Latency Optimization',
    description:
      'Disable LSO, adapter power saving, Interrupt Moderation, and EEE for maximum responsiveness.',
  },
  {
    id: 'disable_power_throttling',
    label: 'Disable CPU Power Throttling',
    description:
      'Prevent Windows from dynamically down-clocking CPU resources for background gaming services.',
  },
  {
    id: 'disable_hpet',
    label: 'Disable HPET (Platform Clock)',
    description:
      'Disable High Precision Event Timer to reduce CPU overhead and input stutters in some games.',
  },
];

export const customizeTweaks: TweakItem[] = [
  {
    id: 'dark_theme',
    label: 'Dark Theme for Windows',
    description: 'Switch the system UI theme to dark so menus and dialogs stay dim.',
  },
  {
    id: 'bing_search_start',
    label: 'Bing Search in Start Menu',
    description: 'Surface Bing results directly in the Start menu search field.',
  },
  {
    id: 'numlock_on_startup',
    label: 'NumLock on Startup',
    description: 'Keep NumLock enabled when Windows boots so the numeric pad is ready.',
  },
  {
    id: 'verbose_logon',
    label: 'Verbose Messages During Logon',
    description: 'Show extended logon and shutdown messages for diagnostics.',
  },
  {
    id: 'start_recommendations',
    label: 'Recommendations in Start Menu',
    description: 'Enable Start menu recommendations for pinned and recent apps.',
  },
  {
    id: 'remove_settings_home',
    label: 'Remove Settings Home Page',
    description: 'Skip the widget-heavy home page when Settings launches.',
  },
  {
    id: 'snap_window',
    label: 'Snap Window',
    description: 'Enable window snapping to screen edges.',
  },
  {
    id: 'snap_assist_flyout',
    label: 'Snap Assist Flyout',
    description: 'Show the flyout UI when using the Snap layout affordances.',
  },
  {
    id: 'snap_assist_suggestion',
    label: 'Snap Assist Suggestion',
    description: 'Suggest windows that can participate in Snap layout.',
  },
  {
    id: 'mouse_acceleration',
    label: 'Disable Mouse Acceleration',
    description:
      'Enforce raw mouse input by disabling Windows pointer acceleration (Enhance Pointer Precision).',
  },
  {
    id: 'sticky_keys',
    label: 'Sticky Keys',
    description: 'Enable sticky keys for multi-key shortcuts without holding multiple keys.',
  },
  {
    id: 'multiplane_overlay',
    label: 'Multiplane Overlay',
    description: 'Configure DirectX multiplane overlays for smoother video playback.',
  },
  {
    id: 'new_outlook',
    label: 'New Outlook',
    description: 'Switch to the modern Outlook experience when opening mail links.',
  },
  {
    id: 'show_hidden_files',
    label: 'Show Hidden Files',
    description: 'Display hidden files in Explorer so nothing is obscured.',
  },
  {
    id: 'show_file_extensions',
    label: 'Show File Extensions',
    description: 'Reveal file-type extensions for all files in Explorer.',
  },
  {
    id: 'disable_search',
    label: 'Disable Search Box/Icon',
    description: 'Remove the search box or search icon from the taskbar.',
  },
  {
    id: 'disable_task_view',
    label: 'Disable Task View Button',
    description: 'Remove the Task View icon from the taskbar.',
  },
  {
    id: 'disable_widgets',
    label: 'Disable Widgets Button',
    description: 'Remove the Widgets icon from the taskbar.',
  },
  {
    id: 'disable_chat',
    label: 'Disable Chat (Teams) Button',
    description: 'Remove the Chat/Teams icon from the taskbar.',
  },
  {
    id: 'center_taskbar_items',
    label: 'Center Taskbar Items',
    description: 'Center icons on the taskbar like Windows 11 defaults.',
  },
  {
    id: 'detailed_bso_d',
    label: 'Detailed BSoD',
    description: 'Enable a more detailed blue screen that captures extra dumps.',
  },
  {
    id: 's3_sleep',
    label: 'S3 Sleep',
    description: 'Use the S3 sleep state for faster suspend/resume behavior.',
  },
  {
    id: 'cross_device_resume',
    label: 'Cross-Device Resume',
    description: 'Allow apps to resume work across devices with synced experiences.',
  },
];

export type ConfigItem = {
  id: string;
  label: string;
  description: string;
};

export const configItems: ConfigItem[] = [
  {
    id: 'dotnet_framework',
    label: 'Enable All .NET Framework (2, 3, 4)',
    description: 'Install/enable the legacy and modern .NET runtimes that some apps still require.',
  },
  {
    id: 'hyperv_virtualization',
    label: 'Hyper-V Virtualization',
    description: "Enable Microsoft's native hypervisor for local virtual machines and containers.",
  },
  {
    id: 'legacy_media',
    label: 'Legacy Media (WMP, DirectPlay)',
    description: 'Bring back the classic Windows Media Player and DirectPlay support.',
  },
  {
    id: 'nfs_network_file_system',
    label: 'NFS – Network File System',
    description: 'Turn on the Windows client for NFS shares in mixed-OS environments.',
  },
  {
    id: 'search_box_web_suggestions_enable',
    label: 'Enable Search Box Web Suggestions (requires Explorer restart)',
    description: 'Show Bing-powered suggestions inside the taskbar search field.',
  },
  {
    id: 'search_box_web_suggestions_disable',
    label: 'Disable Search Box Web Suggestions (requires Explorer restart)',
    description: 'Prevent search from hitting the web so only local results appear.',
  },
  {
    id: 'daily_registry_backup',
    label: 'Enable Daily Registry Backup Task (12:30 AM)',
    description: 'Schedule Windows to snapshot HKLM/HKCU hives every night.',
  },
  {
    id: 'legacy_f8_boot_enable',
    label: 'Enable Legacy F8 Boot Recovery',
    description: 'Restore the old F8 boot menu for troubleshooting and safe mode.',
  },
  {
    id: 'legacy_f8_boot_disable',
    label: 'Disable Legacy F8 Boot Recovery',
    description: 'Keep the modern fast boot experience and skip the old menu.',
  },
  {
    id: 'wsl',
    label: 'Windows Subsystem for Linux',
    description: 'Provision WSL so Linux tooling runs alongside Windows.',
  },
  {
    id: 'windows_sandbox',
    label: 'Windows Sandbox',
    description: 'Enable isolated app testing sandboxes (requires virtualization).',
  },
];

export type FixAction = {
  id: string;
  label: string;
  description: string;
};

export const fixActions: FixAction[] = [
  {
    id: 'autologin',
    label: 'Set Up Autologin',
    description: 'Configure a secure auto-login flow for trusted environments.',
  },
  {
    id: 'reset_windows_update',
    label: 'Reset Windows Update',
    description: 'Re-register Windows Update services and folders.',
  },
  {
    id: 'reset_network',
    label: 'Reset Network',
    description: 'Flush networking stacks and restart adapters.',
  },
  {
    id: 'system_corruption_scan',
    label: 'System Corruption Scan',
    description: 'Run SFC/DISM checks to repair corrupted system files.',
  },
  {
    id: 'winget_reinstall',
    label: 'WinGet Reinstall',
    description: 'Reinstall the Windows Package Manager from the Microsoft Store manifest.',
  },
  {
    id: 'remove_adobe_cc',
    label: 'Remove Adobe Creative Cloud',
    description: 'Uninstall leftover Creative Cloud helpers and telemetry services.',
  },
  {
    id: 'remove_activation_watermark',
    label: 'Remove Activate Windows Watermark',
    description: 'Remove the licensing/activation watermark from the desktop corner.',
  },
];
