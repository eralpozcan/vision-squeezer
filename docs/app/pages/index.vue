<script setup lang="ts">
const { data: page } = await useAsyncData('index', () => queryCollection('landing').path('/').first())
if (!page.value) {
  throw createError({ statusCode: 404, statusMessage: 'Page not found', fatal: true })
}

const title = page.value.seo?.title || page.value.title
const description = page.value.seo?.description || page.value.description

useSeoMeta({
  titleTemplate: '',
  title,
  ogTitle: title,
  description,
  ogDescription: description,
  twitterTitle: title,
  twitterDescription: description
})

defineOgImage('Docs', { title, description })

useHead({
  script: [{
    type: 'application/ld+json',
    innerHTML: JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'SoftwareApplication',
      'name': 'VisionSqueezer',
      'operatingSystem': 'Any',
      'applicationCategory': 'DeveloperApplication',
      'description': 'Library, CLI and MCP server that fits images to a token budget before they reach Claude, GPT, or Gemini. For code, agents and pipelines; it cannot resize images pasted into a chat window.',
      'offers': { '@type': 'Offer', 'price': '0', 'priceCurrency': 'USD' },
      'url': 'https://visionsqueezer.com',
      'sameAs': [
        'https://github.com/eralpozcan/vision-squeezer',
        'https://www.npmjs.com/package/vision-squeezer',
        'https://crates.io/crates/vision-squeezer'
      ]
    })
  }]
})
</script>

<template>
  <ContentRenderer
    v-if="page"
    :value="page"
    :prose="false"
  />
</template>
