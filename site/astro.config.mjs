import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';

// Set SITE_URL in the Cloudflare Pages environment once the final domain is known.
const site = process.env.SITE_URL ?? 'https://cashflow.pages.dev';

export default defineConfig({
  site,
  trailingSlash: 'always',
  build: { format: 'directory' },
  integrations: [
    sitemap({
      i18n: {
        defaultLocale: 'en',
        locales: { en: 'en', fi: 'fi', sv: 'sv', de: 'de', fr: 'fr', es: 'es' },
      },
    }),
  ],
});
