export default defineAppConfig({
  ui: {
    colors: {
      primary: 'indigo',
      neutral: 'slate'
    },
    footer: {
      slots: {
        root: 'border-t border-default',
        left: 'text-sm text-muted'
      }
    }
  },
  seo: {
    siteName: 'VisionSqueezer'
  },
  header: {
    title: 'VisionSqueezer',
    to: '/',
    logo: {
      alt: 'VisionSqueezer',
      light: '/logo.png',
      dark: '/logo.png'
    },
    search: true,
    colorMode: true,
    links: [{
      'icon': 'i-simple-icons-github',
      'to': 'https://github.com/eralpozcan/vision-squeezer',
      'target': '_blank',
      'aria-label': 'VisionSqueezer on GitHub'
    }]
  },
  footer: {
    credits: `Built with 🦀 Rust • Elastic License 2.0 (ELv2) • © ${new Date().getFullYear()} VisionSqueezer`,
    colorMode: false,
    links: [{
      'icon': 'i-simple-icons-github',
      'to': 'https://github.com/eralpozcan/vision-squeezer',
      'target': '_blank',
      'aria-label': 'VisionSqueezer on GitHub'
    }, {
      'icon': 'i-simple-icons-npm',
      'to': 'https://www.npmjs.com/package/vision-squeezer',
      'target': '_blank',
      'aria-label': 'VisionSqueezer on npm'
    }, {
      'icon': 'i-simple-icons-rust',
      'to': 'https://crates.io/crates/vision-squeezer',
      'target': '_blank',
      'aria-label': 'VisionSqueezer on crates.io'
    }]
  },
  toc: {
    title: 'On this page',
    bottom: {
      title: 'Links',
      edit: 'https://github.com/eralpozcan/vision-squeezer/edit/main/docs/content',
      links: [{
        icon: 'i-lucide-star',
        label: 'Star on GitHub',
        to: 'https://github.com/eralpozcan/vision-squeezer',
        target: '_blank'
      }]
    }
  }
})
