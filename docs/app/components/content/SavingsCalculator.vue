<script setup lang="ts">
type Stat = { val: string, pct: string, sub: string, hl: boolean }
type Target = { claude: Stat, gpt4o: Stat, file: { val: string, sub: string }, note: string }
type Source = { desc: string, agnostic: Target, gpt4o: Target, claude: Target }

const benchmarkData: Record<'standard' | 'highres', Source> = {
  standard: {
    desc: 'Optimizing a standard 2400x1670 screenshot.',
    agnostic: {
      claude: { val: '1,150', pct: '-21.5%', sub: '5,344 → 4,194 tokens', hl: true },
      gpt4o: { val: '340', pct: '-30.8%', sub: '1,105 → 765 tokens', hl: true },
      file: { val: '28.6%', sub: '0.5MB → 0.3MB' },
      note: 'When no target is specified, Squeezer reduces file size and mathematically optimizes boundaries to be generally efficient across all models.'
    },
    gpt4o: {
      claude: { val: '1,506', pct: '-28.2%', sub: '5,344 → 3,838 tokens', hl: false },
      gpt4o: { val: 'Maximum', pct: 'Locked', sub: 'Perfectly locked to 6 tiles', hl: true },
      file: { val: '33.6%', sub: '0.5MB → 0.3MB' },
      note: "Targeting GPT-4o perfectly fits the image into a solid 6-tile boundary (2399x1200) mathematically calculated backwards from OpenAI's short-side scaling algorithm."
    },
    claude: {
      claude: { val: '626', pct: '-11.7%', sub: '5,344 → 4,718 tokens', hl: true },
      gpt4o: { val: '0', pct: '0%', sub: '1,105 → 1,105 tokens', hl: false },
      file: { val: '21.3%', sub: '0.5MB → 0.4MB' },
      note: "By targeting Claude, Squeezer preserves the massive 2304x1536 resolution and solely trims solid padding, minimizing token cost via Claude's area-based formula."
    }
  },
  highres: {
    desc: 'Optimizing a massive 4096x3072 photograph (12MP).',
    agnostic: {
      claude: { val: '4,544', pct: '-27.1%', sub: '16,777 → 12,233 tokens', hl: true },
      gpt4o: { val: '0', pct: 'Anomaly', sub: 'OpenAI Grid Paradox Detected', hl: false },
      file: { val: '39.6%', sub: '2.2MB → 1.3MB' },
      note: "Notice the OpenAI Aspect Ratio Anomaly: Removing padding made the image 'wider', which ironically pushes the long-side into a new OpenAI grid row! (Use --model gpt4o to fix)."
    },
    gpt4o: {
      claude: { val: '5,595', pct: '-33.3%', sub: '16,777 → 11,182 tokens', hl: false },
      gpt4o: { val: 'Maximum', pct: 'Locked', sub: 'Grid boundary perfectly contained', hl: true },
      file: { val: '43.2%', sub: '2.2MB → 1.2MB' },
      note: 'By explicitly targeting gpt4o, Squeezer optimizes the boundaries such that the new aspect ratio is safely contained. File footprint shrinks by 43%.'
    },
    claude: {
      claude: { val: '2,360', pct: '-14.1%', sub: '16,777 → 14,417 tokens', hl: true },
      gpt4o: { val: '0', pct: '0%', sub: '765 → 1,105 (Padding trim anomaly)', hl: false },
      file: { val: '31.5%', sub: '2.2MB → 1.5MB' },
      note: 'Squeezer refuses to aggressively downscale (like GPT requires), instead carefully trimming padding to preserve 10+ Megapixels of ultra-fine detail.'
    }
  }
}

const sources = [
  { val: 'standard', label: 'Standard (4MP)' },
  { val: 'highres', label: 'High-Res (12MP)' }
] as const

const targets = [
  { val: 'agnostic', label: 'Agnostic' },
  { val: 'gpt4o', label: 'GPT-4o' },
  { val: 'claude', label: 'Claude' }
] as const

const currentImage = ref<'standard' | 'highres'>('standard')
const currentTarget = ref<'agnostic' | 'gpt4o' | 'claude'>('agnostic')

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
          Optimization Target
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
      {{ desc }} Target: <b class="text-default">{{ targetName }}</b>.
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
        :class="data.gpt4o.hl ? 'border-primary bg-primary/5' : 'border-default'"
      >
        <h4 class="text-xs font-medium text-muted uppercase tracking-wide">
          Tokens Saved (GPT-4o)
        </h4>
        <div class="mt-1 text-2xl font-bold text-default">
          {{ data.gpt4o.val }} <small class="text-base text-muted">({{ data.gpt4o.pct }})</small>
        </div>
        <div class="mt-1 text-xs text-muted">
          {{ data.gpt4o.sub }}
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
