<script lang="ts">
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { downloads } from '$lib/downloads';
    import type { Download } from '$lib/downloadManager';

    let cpuUsage: number = 0;
    let usedMemory: number = 0;
    let totalMemory: number = 0;
    let totalDiskSpace: number = 0;
    let availableDiskSpace: number = 0;

    interface LogEntry {
        timestamp: string;
        level: 'INFO' | 'WARN' | 'ERROR';
        message: string;
    }

    let systemLogs: LogEntry[] = [];

    function formatBytes(bytes: number, decimals = 2) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const dm = decimals < 0 ? 0 : decimals;
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
    }

    function getTimestamp() {
        const d = new Date();
        const pad = (n: number) => String(n).padStart(2, '0');
        return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
    }

    function generateMockLog() {
        const levels = [
            { name: 'INFO', cls: 'info' },
            { name: 'WARN', cls: 'warn' },
            { name: 'ERROR', cls: 'error' }
        ];
        const messages = [
            'Scheduled task ran successfully',
            'Cache warmed: 24 entries',
            'Connection pool at capacity (8/8)',
            'Background sync completed in 312ms',
            'Auth token refreshed',
            'High memory usage detected (72%)',
            'Disk IO throttled temporarily',
            'API rate limit nearing threshold',
            'Configuration reloaded',
            'Retrying message delivery to queue',
        ];
        const level = levels[Math.floor(Math.random() * levels.length)];
        const msg = messages[Math.floor(Math.random() * messages.length)];

        systemLogs = [...systemLogs, { timestamp: getTimestamp(), level: level.name as LogEntry['level'], message: msg }];

        if (systemLogs.length > 100) {
            systemLogs = systemLogs.slice(systemLogs.length - 100);
        }
    }

    onMount(() => {
        const fetchData = async () => {
            try {
                cpuUsage = await invoke('get_cpu_usage');
                usedMemory = await invoke('get_memory_usage');
                totalMemory = await invoke('get_total_memory');
                const [total, available]: [number, number] = await invoke('cleaner_get_drive_info');
                totalDiskSpace = total;
                availableDiskSpace = available;
            } catch (error) {
                console.error('Failed to fetch system info:', error);
            }
        };

        fetchData();

        const intervalId = setInterval(fetchData, 5000);
        const logIntervalId = setInterval(generateMockLog, 3000);

        return () => {
            clearInterval(intervalId);
            clearInterval(logIntervalId);
        };
    });

    $: activeDownloads = $downloads.filter((dl: Download) =>
        dl.status === 'downloading' || dl.status === 'pending' || dl.status === 'queued'
    );
</script>

<div class="main-content">
    <div class="header-card">
        <h1>Welcome back darling! 👋</h1>
        <p>Your system is running smoothly. Here's what's happening today.</p>
    </div>

    <div class="metrics-grid">
        <div class="metric-card">
            <div class="metric-icon">💻</div>
            <div class="metric-top-content">
                <h3 class="metric-title">CPU Usage</h3>
            </div>
            <div class="metric-value-container">
                <span class="metric-value">{cpuUsage.toFixed(1)}</span>
                <span class="metric-unit">%</span>
            </div>
            <p class="metric-subtitle">Current performance</p>
            <span class="metric-trend up metric-trend-style">+0%</span>
        </div>

        <div class="metric-card">
            <div class="metric-icon">🧠</div>
            <div class="metric-top-content">
                <h3 class="metric-title">Memory Usage</h3>
            </div>
            <div class="metric-value-container">
                <span class="metric-value">{formatBytes(usedMemory)}</span>
                <span class="metric-unit"></span>
            </div>
            <p class="metric-subtitle">Used of {formatBytes(totalMemory)}</p>
            <span class="metric-trend up metric-trend-style">+0%</span>
        </div>

        <div class="metric-card">
            <div class="metric-icon">💾</div>
            <div class="metric-top-content">
                <h3 class="metric-title">Disk Space</h3>
            </div>
            <div class="metric-value-container">
                <span class="metric-value">{formatBytes(availableDiskSpace)}</span>
                <span class="metric-unit"></span>
            </div>
            <p class="metric-subtitle">Available of {formatBytes(totalDiskSpace)}</p>
            <span class="metric-trend up metric-trend-style">+0%</span>
        </div>
    </div>

    <div class="secondary-grid">
        <section class="extra-section logs-section">
            <h2 class="downloads-header">System Logs</h2>
            <div class="logs-container" id="logs-container">
                {#each systemLogs as log (log.timestamp + log.message)}
                    <div class="log-item">
                        <span class="log-timestamp">{log.timestamp}</span>
                        <span class="log-level {log.level.toLowerCase()}">{log.level}</span>
                        <span class="log-message">{log.message}</span>
                    </div>
                {/each}
            </div>
        </section>
        <div class="downloads-section">
            <h2 class="downloads-header">Active Downloads</h2>
            <div class="downloads-list">
                {#each activeDownloads as download (download.id)}
                    <div class="download-item">
                        <div class="download-icon">
                            {#if download.status === 'downloading'}🔵
                            {:else if download.status === 'pending'}🟡
                            {:else if download.status === 'queued'}⚪
                            {:else}❓
                            {/if}
                        </div>
                        <div class="download-content">
                            <div class="download-header">
                                <span class="download-name">{download.name}</span>
                                <span class="download-percentage">{download.progress.toFixed(0)}%</span>
                            </div>
                            <div class="progress-bar">
                                <div class="progress-fill" style="width: {download.progress}%"></div>
                            </div>
                            <div class="download-info">
                                <span class="download-size">{download.size}</span>
                            </div>
                        </div>
                    </div>
                {/each}
                {#if activeDownloads.length === 0}
                    <p class="no-active-downloads">No active downloads.</p>
                {/if}
            </div>
        </div>
    </div>
</div>

<style>
    * {
        margin: 0;
        padding: 0;
        box-sizing: border-box;
    }
    
    .main-content {
        margin: 0 auto;
        display: flex;
        flex-direction: column;
        height: 100%;
        flex: 1 1 auto;
        min-height: 0; 
    }
    .header-card {
        background-color: #121212;
        background-image: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
        border: 1px solid var(--avelonia-border);
        border-radius: 0.75rem;
        padding: 25px 0 58px 32px;
        box-shadow: var(--shadow-card);
        transition: var(--transition-smooth);
        margin-bottom: 2rem;
    }
    .header-card:hover {
        transform: translateY(-2px);
        box-shadow: var(--shadow-card), var(--shadow-purple);
    }
    .header-card h1 {
        color: var(--avelonia-text);
        font-size: 1.5rem;
        font-weight: 600;
        margin-bottom: 0.5rem;
    }
    .header-card p {
        color: var(--avelonia-text-muted);
    }

    .metrics-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
    }
    .metric-card {
        background-color: #121212;
        background-image: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
        border: 1px solid var(--avelonia-border);
        border-radius: 0.75rem;
        padding: 1.5rem;
        box-shadow: var(--shadow-card);
        transition: var(--transition-smooth);
        position: relative;
    }
    .metric-card:hover {
        transform: translateY(-2px);
        box-shadow: var(--shadow-card), var(--shadow-purple);
    }
    .metric-36px-div {
        height: 36px; width: 36px;
        margin-bottom: 13px;
        margin-top: 8px;
        background-color: var(--avelonia-border);
    }
    .metric-title {
        color: var(--avelonia-text-muted);
        font-size: 0.875rem;
        font-weight: 500;
        margin: 0;
    }
    .metric-trend {
        position: absolute;
        top: 24px;
        right: 24px;
    }
    .metric-trend-style {
        font-size: 0.75rem;
        font-weight: 500;
        padding: 0.25rem 0.5rem;
        border-radius: 9999px;
    }
    .metric-trend.up {
        color: var(--avelonia-success);
        background-color: hsl(142, 76%, 36%, 0.1);
    }
    .metric-value-container {
        display: flex;
        align-items: baseline;
        gap: 0.25rem;
    }
    .metric-value {
        color: var(--avelonia-text);
        font-size: 1.875rem;
        font-weight: bold;
    }
    .metric-unit {
        color: var(--avelonia-text-muted);
        font-size: 0.875rem;
    }
    .metric-subtitle {
        color: var(--avelonia-text-muted);
        font-size: 0.75rem;
        margin: 0;
    }
    .metric-icon {
        height: 36px;
        width: 36px;
        margin-bottom: 13px;
        margin-top: 8px;
        background-color: var(--avelonia-border);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1.5rem;
        border-radius: 0.5rem;
    }

    .no-active-downloads {
        color: var(--avelonia-text-muted);
        text-align: center;
        padding: 1rem;
    }
    .downloads-section {
        background-color: #121212;
        background-image: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
        border: 1px solid var(--avelonia-border);
        border-radius: 0.75rem;
        padding: 1.5rem;
        box-shadow: var(--shadow-card);
    }
    .secondary-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: 1.5rem;
        margin-bottom: 0; 
        flex: 1 1 auto;
        min-height: 0;
        overflow: hidden; 
        grid-auto-rows: 1fr; 
    }
    .extra-section {
        background-color: #121212;
        background-image: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
        border: 1px solid var(--avelonia-border);
        border-radius: 0.75rem;
        padding: 1.5rem;
        box-shadow: var(--shadow-card);
    }
    .logs-section {
        background-color: #121212;
        background-image: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
    }
    .logs-section,
    .downloads-section {
        display: flex;
        flex-direction: column;
        min-height: 0; 
    }
    .downloads-header {
        color: var(--avelonia-text);
        font-size: 1.125rem;
        font-weight: 600;
        margin-bottom: 1.5rem;
    }
    .downloads-list {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        flex: 1 1 auto;
        min-height: 0;
        overflow: auto;
    }
    .download-item {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        border: 1px solid var(--avelonia-border);
        border-radius: 0.5rem;
        background-color: hsl(220, 13%, 12%, 0.5);
        transition: var(--transition-smooth);
    }
    .download-item:hover {
        background-color: hsl(220, 13%, 15%, 0.8);
        border-color: var(--avelonia-purple);
    }
    .download-icon {
        width: 2rem;
        height: 2rem;
        background-color: var(--avelonia-card);
        border-radius: 0.5rem;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.75rem;
    }
    .download-content {
        flex: 1;
    }
    .download-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 0.5rem;
    }
    .download-name {
        color: var(--avelonia-text);
        font-size: 0.875rem;
        font-weight: 500;
    }
    .download-percentage {
        color: var(--avelonia-text-muted);
        font-size: 0.75rem;
    }
    .progress-bar {
        width: 100%;
        height: 0.25rem;
        background-color: var(--avelonia-border);
        border-radius: 9999px;
        overflow: hidden;
    }
    .progress-fill {
        height: 100%;
        background-color: var(--avelonia-blue);
        border-radius: 9999px;
        transition: width 0.3s ease;
    }
    .download-info {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: 0.25rem;
    }
    .download-size {
        color: var(--avelonia-text-muted);
        font-size: 0.75rem;
    }
    .logs-section h2 {
        font-weight: 600;
        margin-bottom: 1rem;
    }
    .logs-container {
        flex: 1 1 auto;
        min-height: 0;
        overflow: auto;
        padding: 0.5rem;
        border: none; 
        border-radius: 0.5rem;
        background-color: hsl(220, 13%, 12%, 0.35);
        backdrop-filter: blur(2px);
    }
    .log-item {
        display: grid;
        grid-template-columns: 90px 70px 1fr;
        gap: 0.75rem;
        align-items: center;
        padding: 0.5rem 0.25rem;
        border-bottom: none;
    }
    .log-item:last-child {
        border-bottom: none;
    }
    .log-timestamp {
        color: var(--avelonia-text-muted);
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
        font-size: 0.75rem;
    }
    .log-level {
        font-size: 0.7rem;
        font-weight: 600;
        padding: 0.25rem 0.5rem;
        border-radius: 9999px;
        border: none;
        width: fit-content;
        text-align: center;
    }
    .log-level.info {
        color: var(--avelonia-blue);
        background-color: hsla(212, 100%, 50%, 0.1);
    }
    .log-level.warn {
        color: var(--avelonia-warning);
        background-color: hsla(45, 100%, 50%, 0.12);
    }
    .log-level.error {
        color: var(--avelonia-danger);
        background-color: hsla(0, 84%, 60%, 0.12);
    }
    .log-message {
        color: var(--avelonia-text);
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
        font-size: 0.8rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .logs-section .downloads-list {
        display: none;
    }

    .metric-card {
        animation: fade-in 0.3s ease-out;
    }
    .metric-card:nth-child(1) { animation-delay: 0.1s; }
    .metric-card:nth-child(2) { animation-delay: 0.2s; }
    .metric-card:nth-child(3) { animation-delay: 0.3s; }
    .downloads-section {
        animation: fade-in 0.3s ease-out 0.4s both;
    }
    .extra-section {
        animation: fade-in 0.3s ease-out 0.2s both;
    }

    @keyframes fade-in {
        from {
            opacity: 0;
            transform: translateY(20px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    @media (max-width: 768px) {
        .metrics-grid {
            grid-template-columns: 1fr;
            gap: 1rem;
        }
        
        .metric-card, .downloads-section {
            padding: 1rem;
        }
        .secondary-grid {
            grid-template-columns: 1fr;
            grid-template-rows: auto; 
            gap: 1rem;
            flex: 0 0 auto;
            min-height: auto;
            overflow: visible;
        }
        .extra-section {
            padding: 1rem;
        }
        .logs-container {
            height: 220px; 
            overflow: auto;
        }
        .downloads-list {
            max-height: 240px; 
            overflow: auto;
        }
    }
    @media (min-width: 768px) and (max-width: 1199px) {
        .secondary-grid {
            grid-template-columns: 1fr 1fr;
            grid-auto-rows: 1fr; 
        }
    }
    @media (min-width: 1200px) {
        .secondary-grid {
            grid-template-columns: 2fr 1fr;
            grid-auto-rows: 1fr; 
        }
    }
</style>