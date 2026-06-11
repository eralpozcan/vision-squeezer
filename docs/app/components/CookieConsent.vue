<script setup lang="ts">
const visible = ref(false)

const { load } = useScriptUmamiAnalytics({
  websiteId: '2d1bce07-25fc-40c0-8853-34d064d67101',
  scriptOptions: {
    trigger: 'manual'
  }
})

function consent(choice: 'accepted' | 'declined') {
  localStorage.setItem('vs_cookie_consent', choice)
  visible.value = false
  if (choice === 'accepted') load()
}

onMounted(() => {
  const stored = localStorage.getItem('vs_cookie_consent')
  if (stored === 'accepted') load()
  else if (!stored) visible.value = true
})
</script>

<template>
  <div
    v-if="visible"
    class="fixed inset-x-0 bottom-0 z-50 flex flex-wrap items-center justify-between gap-4 border-t border-default bg-default/95 px-6 py-4 backdrop-blur"
  >
    <p class="flex-1 min-w-50 text-sm text-muted">
      We use anonymous analytics (<a
        href="https://umami.is"
        target="_blank"
        rel="noopener"
        class="text-primary"
      >Umami</a>) to understand how people discover VisionSqueezer. No personal data collected.
    </p>
    <div class="flex shrink-0 gap-2">
      <UButton
        color="neutral"
        variant="outline"
        size="sm"
        @click="consent('declined')"
      >
        Decline
      </UButton>
      <UButton
        color="primary"
        size="sm"
        @click="consent('accepted')"
      >
        Accept
      </UButton>
    </div>
  </div>
</template>
