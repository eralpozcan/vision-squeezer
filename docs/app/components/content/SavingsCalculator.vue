<script setup lang="ts">
type Stat = { val: string, pct: string, sub: string, hl: boolean }
type Target = { claude: Stat, gpt6: Stat, file: { val: string, sub: string }, note: string }
type SourceKey = 'standard' | 'highres'
type TargetKey = 'off' | 'b1600' | 'b1000'
type Source = { desc: string } & Record<TargetKey, Target>

const benchmarkData: Record<SourceKey, Source> = {
  standard: {
    desc: 'Optimizing a 2400x1670 photograph (4MP).',
    off: {
      claude: { val: '604', pct: '-13%', sub: '4,674 → 4,070 tokens', hl: true },
      gpt6: { val: '0', pct: '0%', sub: '2,903 → 2,942 tokens', hl: false },
      file: { val: '29%', sub: '0.5MB → 0.4MB' },
      note: 'No budget (the pre-0.7 default): Squeezer snaps to the provider grid and strips padding. Large photos barely change because providers already downscale oversized images themselves.'
    },
    b1600: {
      claude: { val: '3,090', pct: '-66%', sub: '4,674 → 1,584 tokens', hl: true },
      gpt6: { val: '1,441', pct: '-50%', sub: '2,903 → 1,462 tokens', hl: true },
      file: { val: '67%', sub: '0.5MB → 0.2MB' },
      note: 'Budget 1600 (the MCP default): the image is downscaled until Claude\'s estimate fits. Composition, colour and large text survive; fine detail goes first.'
    },
    b1000: {
      claude: { val: '3,686', pct: '-79%', sub: '4,674 → 988 tokens', hl: true },
      gpt6: { val: '1,964', pct: '-68%', sub: '2,903 → 939 tokens', hl: true },
      file: { val: '79%', sub: '0.5MB → 0.1MB' },
      note: 'Budget 1000: more aggressive. Body text in a retina code screenshot stayed legible in testing; around 600 it did not.'
    }
  },
  highres: {
    desc: 'Optimizing a 4096x3072 photograph (12MP).',
    off: {
      claude: { val: '0', pct: '0%', sub: '4,661 → 4,698 tokens', hl: false },
      gpt6: { val: '18', pct: '-1%', sub: '2,942 → 2,924 tokens', hl: true },
      file: { val: '40%', sub: '2.3MB → 1.4MB' },
      note: 'No budget (the pre-0.7 default): Squeezer snaps to the provider grid and strips padding. Large photos barely change because providers already downscale oversized images themselves.'
    },
    b1600: {
      claude: { val: '3,097', pct: '-66%', sub: '4,661 → 1,564 tokens', hl: true },
      gpt6: { val: '1,466', pct: '-50%', sub: '2,942 → 1,476 tokens', hl: true },
      file: { val: '88%', sub: '2.3MB → 0.3MB' },
      note: 'Budget 1600 (the MCP default): the image is downscaled until Claude\'s estimate fits. Composition, colour and large text survive; fine detail goes first.'
    },
    b1000: {
      claude: { val: '3,662', pct: '-79%', sub: '4,661 → 999 tokens', hl: true },
      gpt6: { val: '1,991', pct: '-68%', sub: '2,942 → 951 tokens', hl: true },
      file: { val: '92%', sub: '2.3MB → 0.2MB' },
      note: 'Budget 1000: more aggressive. Body text in a retina code screenshot stayed legible in testing; around 600 it did not.'
    }
  }
}

const sources = [
  { val: 'standard', label: 'Standard (4MP)' },
  { val: 'highres', label: 'High-Res (12MP)' }
] as const

const targets = [
  { val: 'off', label: 'No budget' },
  { val: 'b1600', label: '1600 tokens' },
  { val: 'b1000', label: '1000 tokens' }
] as const

const currentImage = ref<SourceKey>('standard')
const currentTarget = ref<TargetKey>('b1600')

const data = computed(() => benchmarkData[currentImage.value][currentTarget.value])
const targetName = computed(() => targets.find(t => t.val === currentTarget.value)!.label)
const desc = computed(() => benchmarkData[currentImage.value].desc)
</script>

<template>
  <div class="rounded-xl border border-default bg-elevated/50 p-6 my-6">
    <div class="grid sm:grid-cols-2 gap-6">
      <div>
        <p class="text-sm font-medium text-muted mb-2">
          Select Image Source
        </p>
        <div class="flex gap-1 rounded-lg bg-default p-1">
          <button
            v-for="s in sources"
            :key="s.val"
            class="flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors"
            :class="currentImage === s.val ? 'bg-primary text-inverted' : 'text-muted hover:text-default'"
            @click="currentImage = s.val"
          >
            {{ s.label }}
          </button>
        </div>
      </div>
      <div>
        <p class="text-sm font-medium text-muted mb-2">
          Token budget
        </p>
        <div class="flex gap-1 rounded-lg bg-default p-1">
          <button
            v-for="t in targets"
            :key="t.val"
            class="flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors"
            :class="currentTarget === t.val ? 'bg-primary text-inverted' : 'text-muted hover:text-default'"
            @click="currentTarget = t.val"
          >
            {{ t.label }}
          </button>
        </div>
      </div>
    </div>

    <hr class="my-6 border-default">

    <p class="text-sm text-muted mb-4">
      {{ desc }} Budget: <b class="text-default">{{ targetName }}</b>.
    </p>

    <div class="grid sm:grid-cols-3 gap-4">
      <div
        class="rounded-lg border p-4 transition-colors"
        :class="data.claude.hl ? 'border-primary bg-primary/5' : 'border-default'"
      >
        <h4 class="text-xs font-medium text-muted uppercase tracking-wide">
          Tokens Saved (Claude)
        </h4>
        <div class="mt-1 text-2xl font-bold text-default">
          {{ data.claude.val }} <small class="text-base text-muted">({{ data.claude.pct }})</small>
        </div>
        <div class="mt-1 text-xs text-muted">
          {{ data.claude.sub }}
        </div>
      </div>
      <div
        class="rounded-lg border p-4 transition-colors"
        :class="data.gpt6.hl ? 'border-primary bg-primary/5' : 'border-default'"
      >
        <h4 class="text-xs font-medium text-muted uppercase tracking-wide">
          Tokens Saved (GPT-6)
        </h4>
        <div class="mt-1 text-2xl font-bold text-default">
          {{ data.gpt6.val }} <small class="text-base text-muted">({{ data.gpt6.pct }})</small>
        </div>
        <div class="mt-1 text-xs text-muted">
          {{ data.gpt6.sub }}
        </div>
      </div>
      <div class="rounded-lg border border-default p-4">
        <h4 class="text-xs font-medium text-muted uppercase tracking-wide">
          File Size Reduced
        </h4>
        <div class="mt-1 text-2xl font-bold text-default">
          {{ data.file.val }} <small class="text-base text-muted">Smaller</small>
        </div>
        <div class="mt-1 text-xs text-muted">
          {{ data.file.sub }}
        </div>
      </div>
    </div>

    <p class="mt-4 text-xs text-muted italic">
      {{ data.note }}
    </p>
  </div>
</template>
