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
  ogDescription: description
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
      'description': 'LLM-native image optimization middleware and MCP server that mathematically snaps images to exact grid boundaries for Claude, GPT, and Gemini to reduce token usage.',
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
