// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: [
    '@nuxt/eslint',
    '@nuxt/image',
    '@nuxt/scripts',
    '@nuxt/ui',
    '@nuxt/content',
    '@nuxtjs/robots',
    '@nuxtjs/sitemap',
    'nuxt-og-image',
    'nuxt-llms',
    '@nuxtjs/mcp-toolkit'
  ],

  devtools: {
    enabled: true
  },

  app: {
    head: {
      link: [
        { rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' },
        { rel: 'icon', type: 'image/png', sizes: '16x16', href: '/favicon-16x16.png' },
        { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32x32.png' },
        { rel: 'apple-touch-icon', sizes: '180x180', href: '/apple-touch-icon.png' },
        { rel: 'manifest', href: '/site.webmanifest' }
      ],
      meta: [
        { name: 'theme-color', content: '#6366f1' }
      ]
    }
  },

  css: ['~/assets/css/main.css'],

  site: {
    url: 'https://visionsqueezer.com',
    name: 'VisionSqueezer'
  },

  content: {
    build: {
      markdown: {
        toc: {
          searchDepth: 1
        }
      }
    }
  },

  routeRules: {
    '/': { swr: 3600, prerender: false }
  },

  experimental: {
    asyncContext: true
  },

  compatibilityDate: '2024-07-11',

  nitro: {
    prerender: {
      // Seed the crawl from a content page, not '/'. The homepage is served
      // SWR (see routeRules) so its VersionBadge re-fetches the latest release
      // at request time instead of freezing on the build-time tag. The sidebar
      // nav on every content page cross-links the rest, so one seed is enough.
      routes: [
        '/getting-started'
      ],
      crawlLinks: true,
      autoSubfolderIndex: false
    }
  },

  vite: {
    optimizeDeps: {
      include: ['@vueuse/core']
    }
  },

  eslint: {
    config: {
      stylistic: {
        commaDangle: 'never',
        braceStyle: '1tbs'
      }
    }
  },

  icon: {
    provider: 'iconify'
  },

  llms: {
    domain: 'https://visionsqueezer.com',
    title: 'VisionSqueezer',
    description: 'LLM-native image optimization middleware & MCP server. Reduces vision model token consumption by mathematically snapping images to provider-specific tile boundaries.',
    full: {
      title: 'VisionSqueezer — Full Documentation',
      description: 'Complete technical reference for VisionSqueezer: provider math, CLI, MCP server, Python bindings, and sandbox operations.'
    },
    sections: [
      {
        title: 'Getting Started',
        contentCollection: 'docs',
        contentFilters: [
          { field: 'path', operator: 'LIKE', value: '/getting-started%' }
        ]
      },
      {
        title: 'CLI',
        contentCollection: 'docs',
        contentFilters: [
          { field: 'path', operator: 'LIKE', value: '/cli%' }
        ]
      },
      {
        title: 'Providers',
        contentCollection: 'docs',
        contentFilters: [
          { field: 'path', operator: 'LIKE', value: '/providers%' }
        ]
      },
      {
        title: 'Guides',
        contentCollection: 'docs',
        contentFilters: [
          { field: 'path', operator: 'LIKE', value: '/guides%' }
        ]
      }
    ]
  },

  mcp: {
    name: 'VisionSqueezer Docs'
  },

  ogImage: {
    zeroRuntime: true
  }
})
