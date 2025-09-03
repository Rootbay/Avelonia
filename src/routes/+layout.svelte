<script lang="ts">
  import { icons } from "$lib/icons";
  import { page } from '$app/stores';
  import '../app.css';
</script>

<div class="app-container">
  <nav class="sidebar">
    <div class="profile-section">
      <img src="/favicon.png" alt="Avelonia Logo" class="profile-pic" />
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
        </a>
      </li>
      <li>
        <a href="/cleaner" class:active-link={$page.url.pathname === '/cleaner'}>
          <div class="icon">{@html icons.Cleaner}</div>
          <span>Cleaner</span>
        </a>
      </li>
      <li>
        <a href="/eraser" class:active-link={$page.url.pathname === '/eraser'}>
          <div class="icon">{@html icons.Eraser}</div>
          <span>Eraser</span>
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
    background-color: var(--color-primary);
  }

  .sidebar {
    width: 250px;
    background-color: var(--color-secondary);
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

  .content-area {
    flex-grow: 1;
    padding: 40px 30px;
    overflow-y: auto;
  }

  :root {
    --color-primary: #0f0f0f;
    --color-secondary: #121212;
    --color-accent: #C64143;
    font-family: "Inter", Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 24px;
    font-weight: 400;

    color: #0f0f0f;
    background-color: #f6f6f6;

    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
  }
</style>