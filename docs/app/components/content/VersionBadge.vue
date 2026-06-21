<script setup lang="ts">
const props = defineProps<{ fallback?: string }>()
const fallback = computed(() => props.fallback || 'v0.6.0')

// Resolved during SSR/prerender so the real tag is baked into the HTML —
// no post-hydration fetch, no stale-version flash.
const { data: version } = await useAsyncData(
  'latest-release',
  async () => {
    try {
      const data = await $fetch<{ tag_name?: string }>(
        'https://api.github.com/repos/eralpozcan/vision-squeezer/releases/latest'
      )
      return data?.tag_name || fallback.value
    } catch {
      return fallback.value
    }
  },
  { default: () => fallback.value }
)
</script>

<template>
  <UBadge
    color="primary"
    variant="subtle"
    size="lg"
    class="rounded-full"
  >
    {{ version }} Released 🚀
  </UBadge>
</template>
