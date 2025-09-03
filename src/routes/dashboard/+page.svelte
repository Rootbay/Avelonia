<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Avelonia Dashboard</title>
    <meta name="description" content="Avelonia Private Suite Dashboard" />
    <meta name="author" content="Avelonia" />
    
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        :root {
            /* Avelonia Dark Theme Design System */
            --background: hsl(220, 13%, 9%);
            --foreground: hsl(220, 9%, 85%);
            --card: hsl(220, 13%, 12%);
            --card-foreground: hsl(220, 9%, 85%);
            --primary: hsl(268, 100%, 70%);
            --primary-foreground: hsl(220, 13%, 9%);
            --secondary: hsl(220, 13%, 15%);
            --secondary-foreground: hsl(220, 9%, 85%);
            --muted: hsl(220, 13%, 15%);
            --muted-foreground: hsl(220, 9%, 46%);
            --success: hsl(142, 76%, 36%);
            --success-foreground: hsl(220, 9%, 85%);
            --border: hsl(220, 13%, 18%);
            --avelonia-purple: hsl(268, 100%, 70%);
            --avelonia-dark: hsl(220, 13%, 9%);
            --avelonia-darker: hsl(220, 13%, 6%);
            --avelonia-card: hsl(220, 13%, 12%);
            --avelonia-border: hsl(220, 13%, 18%);
            --avelonia-text: hsl(220, 9%, 85%);
            --avelonia-text-muted: hsl(220, 9%, 46%);
            --avelonia-success: hsl(142, 76%, 36%);
            --avelonia-blue: hsl(212, 100%, 50%);
            --avelonia-warning: hsl(45, 100%, 50%);
            --avelonia-danger: hsl(0, 84%, 60%);
            --gradient-purple: linear-gradient(135deg, hsl(268, 100%, 70%) 0%, hsl(268, 100%, 60%) 100%);
            --gradient-card: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
            --shadow-purple: 0 10px 30px -10px hsl(268, 100%, 70%, 0.3);
            --shadow-card: 0 4px 6px -1px hsl(220, 13%, 6%, 0.4);
            --transition-smooth: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }

        html {
            height: 100%;
            overflow: hidden; /* lock page scroll */
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background-color: var(--avelonia-dark);
            color: var(--avelonia-text);
            height: 100dvh; /* ensure full viewport height */
            padding: 2rem;
            overflow: hidden; /* prevent page scroll; internal areas will scroll */
        }

        /* Main Content Styles */
        .main-content {
            margin: 0 auto;
            display: flex;
            flex-direction: column;
            height: 100%;
            flex: 1 1 auto;
            min-height: 0; /* allow children to shrink without forcing page scroll */
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

        .metric-header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
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
            margin-bottom: 0; /* avoid adding extra height that triggers body scroll */
            /* Fill remaining vertical space under header + metrics */
            flex: 1 1 auto;
            min-height: 0;
            overflow: hidden; /* prevent grid from scrolling; let inner panes scroll */
            grid-auto-rows: 1fr; /* make the single row fill available height */
        }

        .extra-section {
            background-color: #121212;
            background-image: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
            border: 1px solid var(--avelonia-border);
            border-radius: 0.75rem;
            padding: 1.5rem;
            box-shadow: var(--shadow-card);
        }

        /* Ensure logs-section uses the same background treatment */
        .logs-section {
            background-color: #121212;
            background-image: linear-gradient(to bottom right, #121212 0%, #16171A 43%, #121316 100%);
        }

        /* Make sections flex columns so inner content can fill leftover height */
        .logs-section,
        .downloads-section {
            display: flex;
            flex-direction: column;
            min-height: 0; /* allow children to shrink */
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
            /* Fill remaining height in downloads section */
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

        /* Logs styling */
        .logs-section h2 {
            font-weight: 600;
            margin-bottom: 1rem;
        }

        .logs-container {
            /* Fill remaining height in logs section */
            flex: 1 1 auto;
            min-height: 0;
            overflow: auto;
            padding: 0.5rem;
            border: none; /* keep lines removed */
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

        /* (spacing reverted; keep default gaps) */

        .log-message {
            color: var(--avelonia-text);
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
            font-size: 0.8rem;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        /* Hide leftover list block inside logs section (cleanup) */
        .logs-section .downloads-list { display: none; }

        /* Animations */
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

        @media (max-width: 768px) {
            html {
                height: auto;
                overflow: auto; /* allow page scroll on small screens */
            }

            body {
                padding: 1rem;
                height: auto;
                min-height: 100dvh;
                overflow: auto; /* enable scrolling when content stacks */
            }
            
            .metrics-grid {
                grid-template-columns: 1fr;
                gap: 1rem;
            }

            .header h1 {
                font-size: 1.25rem;
            }

            .metric-card, .downloads-section {
                padding: 1rem;
            }

            .secondary-grid {
                grid-template-columns: 1fr;
                grid-template-rows: auto; /* natural height per section on mobile */
                gap: 1rem;
                /* Allow page to scroll instead of locking height */
                flex: 0 0 auto;
                min-height: auto;
                overflow: visible;
            }

            .extra-section {
                padding: 1rem;
            }

            .logs-container {
                height: 220px; /* fixed height on small screens */
                overflow: auto;
            }

            .downloads-list {
                max-height: 240px; /* avoid overflowing viewport */
                overflow: auto;
            }
        }

        /* Medium screens: 2 equal columns */
        @media (min-width: 768px) and (max-width: 1199px) {
            .secondary-grid {
                grid-template-columns: 1fr 1fr;
                grid-auto-rows: 1fr; /* single row fills height */
            }
        }

        /* Large screens: logs wider than downloads */
        @media (min-width: 1200px) {
            .secondary-grid {
                grid-template-columns: 2fr 1fr;
                grid-auto-rows: 1fr; /* single row fills height */
            }
        }
    </style>
</head>
<body>
    <!-- Main Content -->
    <div class="main-content">
        <!-- Header -->
        <div class="header-card">
            <h1>Welcome back darling! 👋</h1>
            <p>Your system is running smoothly. Here's what's happening today.</p>
        </div>

        <!-- Metrics Grid -->
        <div class="metrics-grid">
            <div class="metric-card">
              <div class="metric-36px-div"></div>
                <div class="metric-top-content">
                    <h3 class="metric-title">Memory Cleared</h3>
                </div>
                <div class="metric-value-container">
                    <span class="metric-value">99.867</span>
                    <span class="metric-unit">KB</span>
                </div>
                <p class="metric-subtitle">Last cleanup: 2 min ago</p>
                <span class="metric-trend up metric-trend-style">+12%</span>
            </div>
            
            <div class="metric-card">
              <div class="metric-36px-div"></div>
                <div class="metric-top-content">
                    <h3 class="metric-title">Download Speed</h3>
                </div>
                <div class="metric-value-container">
                    <span class="metric-value">88</span>
                    <span class="metric-unit">Mbps</span>
                </div>
                <p class="metric-subtitle">Current average</p>
                <span class="metric-trend up metric-trend-style">+12%</span>
            </div>
            
            <div class="metric-card">
              <div class="metric-36px-div"></div>
                <div class="metric-top-content">
                    <h3 class="metric-title">CPU Usage</h3>
                </div>
                <div class="metric-value-container">
                    <span class="metric-value">34</span>
                    <span class="metric-unit">%</span>
                </div>
                <p class="metric-subtitle">Current performance</p>
                <span class="metric-trend up metric-trend-style">+12%</span>
            </div>
        </div>

        <!-- Secondary grid: extra section (2 cols) + downloads (1 col) -->
        <div class="secondary-grid">
            <section class="extra-section logs-section">
                <h2 class="downloads-header">System Logs</h2>
                <div class="logs-container" id="logs-container">
                    <div class="log-item">
                        <span class="log-timestamp">12:00:01</span>
                        <span class="log-level info">INFO</span>
                        <span class="log-message">Boot sequence initialized</span>
                    </div>
                    <div class="log-item">
                        <span class="log-timestamp">12:00:03</span>
                        <span class="log-level info">INFO</span>
                        <span class="log-message">Loaded config: profile=default, mode=production</span>
                    </div>
                    <div class="log-item">
                        <span class="log-timestamp">12:00:05</span>
                        <span class="log-level warn">WARN</span>
                        <span class="log-message">Network latency slightly elevated (avg 92ms)</span>
                    </div>
                    <div class="log-item">
                        <span class="log-timestamp">12:00:10</span>
                        <span class="log-level error">ERROR</span>
                        <span class="log-message">Retrying connection to telemetry endpoint (attempt 2)</span>
                    </div>
                    <div class="log-item">
                        <span class="log-timestamp">12:00:12</span>
                        <span class="log-level info">INFO</span>
                        <span class="log-message">Service health check passed</span>
                    </div>
                </div>
                <div class="downloads-list">
                    <div class="download-item">
                        <div class="download-icon">ℹ️</div>
                        <div class="download-content">
                            <div class="download-header">
                                <span class="download-name">Status</span>
                                <span class="download-percentage">OK</span>
                            </div>
                            <div class="progress-bar">
                                <div class="progress-fill" style="width: 100%"></div>
                            </div>
                            <div class="download-info">
                                <span class="download-size">All systems nominal</span>
                            </div>
                        </div>
                    </div>
                </div>
            </section>
        <!-- Active Downloads Section -->
        <div class="downloads-section">
            <h2 class="downloads-header">Active Downloads</h2>
            <div class="downloads-list">
                <div class="download-item">
                    <div class="download-icon">🔵</div>
                    <div class="download-content">
                        <div class="download-header">
                            <span class="download-name">Steam.exe</span>
                            <span class="download-percentage">78%</span>
                        </div>
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: 78%"></div>
                        </div>
                        <div class="download-info">
                            <span class="download-size">54 mb/s</span>
                        </div>
                    </div>
                </div>
                
                <div class="download-item">
                    <div class="download-icon">⚫</div>
                    <div class="download-content">
                        <div class="download-header">
                            <span class="download-name">Epic games.exe</span>
                            <span class="download-percentage">65%</span>
                        </div>
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: 65%"></div>
                        </div>
                        <div class="download-info">
                            <span class="download-size">44 mb/s</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        </div>
    </div>

    <script>
        // Simulate real-time progress updates
        document.addEventListener('DOMContentLoaded', function() {
            function updateProgress() {
                const progressBars = document.querySelectorAll('.progress-fill');
                const percentages = document.querySelectorAll('.download-percentage');
                
                progressBars.forEach((bar, index) => {
                    let currentProgress = parseInt(bar.style.width);
                    if (currentProgress < 100) {
                        const newProgress = Math.min(currentProgress + Math.random() * 2, 100);
                        bar.style.width = newProgress + '%';
                        percentages[index].textContent = Math.round(newProgress) + '%';
                    }
                });
            }

            // Update progress every 2 seconds
            setInterval(updateProgress, 2000);

            // Simulate appending system logs
            const logsContainer = document.getElementById('logs-container');
            if (logsContainer) {
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

                function timestamp() {
                    const d = new Date();
                    const pad = (n) => String(n).padStart(2, '0');
                    return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
                }

                function addLog() {
                    const level = levels[Math.floor(Math.random() * levels.length)];
                    const msg = messages[Math.floor(Math.random() * messages.length)];

                    const item = document.createElement('div');
                    item.className = 'log-item';

                    const ts = document.createElement('span');
                    ts.className = 'log-timestamp';
                    ts.textContent = timestamp();

                    const lvl = document.createElement('span');
                    lvl.className = `log-level ${level.cls}`;
                    lvl.textContent = level.name;

                    const text = document.createElement('span');
                    text.className = 'log-message';
                    text.textContent = msg;

                    item.appendChild(ts);
                    item.appendChild(lvl);
                    item.appendChild(text);

                    const atBottom = Math.abs(logsContainer.scrollHeight - logsContainer.clientHeight - logsContainer.scrollTop) < 4;
                    logsContainer.appendChild(item);

                    // Keep list size manageable
                    if (logsContainer.children.length > 100) {
                        logsContainer.removeChild(logsContainer.firstElementChild);
                    }

                    // Auto-scroll if already at bottom
                    if (atBottom) {
                        logsContainer.scrollTop = logsContainer.scrollHeight;
                    }
                }

                setInterval(addLog, 3000);
            }
        });
    </script>
</body>
</html>


