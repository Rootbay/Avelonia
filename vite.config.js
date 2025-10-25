import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import viteCompression from 'vite-plugin-compression';
import { VitePWA } from 'vite-plugin-pwa';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(({ command, mode }) => {
  /** @type {import('vite').PluginOption[]} */
  const plugins = [
    sveltekit(),
    viteCompression({ algorithm: 'brotliCompress', ext: '.br' }),
    viteCompression({ algorithm: 'gzip', ext: '.gz' }),
  ];

  if (command === 'build') {
    plugins.push(
      VitePWA({
        disable: false,
        registerType: 'autoUpdate',
        includeAssets: ['favicon.png'],
        workbox: {
          runtimeCaching: [
            {
              urlPattern: ({ request }) => request.destination === 'document',
              handler: 'NetworkFirst',
              options: { cacheName: 'html-cache', expiration: { maxEntries: 10 } },
            },
            {
              urlPattern: ({ request }) =>
                ['style', 'script', 'worker'].includes(request.destination),
              handler: 'StaleWhileRevalidate',
              options: { cacheName: 'assets-cache', expiration: { maxEntries: 60 } },
            },
            {
              urlPattern: ({ request }) => request.destination === 'image',
              handler: 'StaleWhileRevalidate',
              options: { cacheName: 'images-cache', expiration: { maxEntries: 60 } },
            },
          ],
        },
        manifest: {
          name: 'Avelonia',
          short_name: 'Avelonia',
          theme_color: '#0f1115',
          background_color: '#0f1115',
          display: 'standalone',
          icons: [
            { src: 'favicon.png', sizes: '192x192', type: 'image/png' },
            { src: 'favicon.png', sizes: '512x512', type: 'image/png' },
          ],
        },
      })
    );
  }

  return {
    plugins,
    clearScreen: false,
    optimizeDeps: {
      exclude: ['@tauri-apps/api', '@tauri-apps/plugin-opener', '@tauri-apps/plugin-dialog'],
    },
    server: {
      port: 5174,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: 'ws',
            host,
            port: 5175,
          }
        : undefined,
      watch: {
        ignored: ['**/src-tauri/**'],
      },
    },
    build: {
      cssCodeSplit: true,
      assetsInlineLimit: 4096,
      chunkSizeWarningLimit: 700,
      target: 'es2020',
      minify: 'esbuild',
      rollupOptions: {
        external: ['@tauri-apps/api/tauri'],
        output: {
          /** @param {any} assetInfo */
          assetFileNames: (assetInfo) => {
            if (assetInfo.name.endsWith('.css')) {
              return 'assets/app.[ext]';
            }
            return 'assets/[name].[ext]';
          },
        },
      },
      esbuild: { drop: ['console', 'debugger'] },
    },
  };
});
