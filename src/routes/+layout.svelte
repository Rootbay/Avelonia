<script lang="ts">
  import { icons } from "$lib/icons";
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { initDownloadListener, disposeDownloadListener } from '$lib/downloadManager';
  import { downloads } from '$lib/downloads';
  import '../app.css';

  onMount(() => {
    initDownloadListener();
  });
  onDestroy(() => {
    disposeDownloadListener();
  });

  const activeCount = $derived($downloads.filter(d => d.status === 'downloading' || d.status === 'pending' || d.status === 'queued').length);
</script>

<div class="app-container">
  <nav class="sidebar">
    <div class="profile-section">
      <img src="/favicon.png" alt="Avelonia Logo" class="profile-pic" width="50" height="50" decoding="async" loading="eager" fetchpriority="low" />
      <p class="app-name">Avelonia</p>
    </div>

    

    <ul class="nav-links">
      <li>
        <a href="/dashboard" class:active-link={$page.url.pathname === '/dashboard'}>
          <div class="icon">{@html icons.Dashboard}</div>
          <span>Dashboard</span>
        </a>
      </li>
      <li>
        <a href="/optimize" class:active-link={$page.url.pathname === '/optimize'}>
          <div class="icon">{@html icons.Optimize}</div>
          <span>Optimize</span>
        </a>
      </li>
      <li>
        <a href="/downloader" class:active-link={$page.url.pathname === '/downloader'}>
          <div class="icon">{@html icons.Downloader}</div>
          <span>Downloader</span>
          {#if activeCount > 0}
            <span class="badge" aria-label={`Active downloads: ${activeCount}`}>{activeCount}</span>
          {/if}
        </a>
      </li>
      <li>
        <a href="/cleaner" class:active-link={$page.url.pathname === '/cleaner'}>
          <div class="icon">{@html icons.Cleaner}</div>
          <span>Cleaner</span>
        </a>
      </li>
      
    </ul>
  </nav>

  <main class="content-area">
    <slot />
  </main>
</div>

<style>
  .app-container {
    display: flex;
    height: 100vh;
    background-color: var(--background);
  }

  .sidebar {
    width: 250px;
    flex: 0 0 250px; /* prevent flex shrink/expand */
    min-width: 250px;
    background-color: var(--secondary);
    color: white;
    padding: 0;
    box-shadow: 2px 0 5px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
  }

  .profile-section {
    display: flex;
    align-items: center;
    height: 54px;
    margin: 20px;
    margin-bottom: 0;
  }

  .profile-pic {
    width: 50px;
    height: 50px;
    border-radius: 50%;
    object-fit: cover;
    margin-right: 15px;
  }

  .app-name {
    font-family: "Inter", sans-serif;
    font-weight: 500;
    font-size: 18px;
    color: var(--white);
    margin: 0;
  }

  .nav-links {
    list-style: none;
    padding: 0;
    margin: auto 0;
    flex-grow: 1;
  }

  .nav-links li {
    margin-bottom: 4px;
  }

  .nav-links a {
    display: flex;
    align-items: center;
    color: white;
    text-decoration: none;
    padding: 10px 20px;
    transition: background-color 0.3s ease, border-right-color 0.3s ease;
    border-right: 2px solid transparent;
  }

  .nav-links a:hover {
    background-color: var(--color-primary);
    border-right-color: var(--color-accent);
  }

  .nav-links a.active-link {
    background-color: var(--color-primary);
    border-right-color: var(--color-accent);
  }

  .nav-links a span {
    margin-left: 10px;
    font-family: "Inter", sans-serif;
    font-weight: 400;
    font-size: 15px;
    color: var(--white);
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
  }

  .badge {
    margin-left: auto;
    background: var(--avelonia-purple);
    color: #000;
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 12px;
    font-weight: 600;
  }

  .content-area {
    flex-grow: 1;
    padding: 40px 30px;
    overflow-y: auto;
    scrollbar-gutter: stable; /* avoid layout shift when scrollbar appears */
    min-width: 0; /* prevent flex overflow pushing sidebar */
  }

  
</style>
