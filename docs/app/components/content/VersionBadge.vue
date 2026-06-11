<script setup lang="ts">
const props = defineProps<{ fallback?: string }>()
const version = ref(props.fallback || 'v0.3.5')

onMounted(async () => {
  try {
    const res = await fetch('https://api.github.com/repos/eralpozcan/vision-squeezer/releases/latest')
    const data = await res.json()
    if (data?.tag_name) version.value = data.tag_name
  } catch {
    // keep fallback
  }
})
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
