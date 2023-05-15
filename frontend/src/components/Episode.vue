<script setup lang="ts">
import { computed } from "vue";
import { type EpisodeData } from "../types"

const props = defineProps<EpisodeData>()

const docs_link = computed(() => {
  return `https://docs.google.com/document/d/${props.docs_id}`
})

</script>

<template>
  <p class="episode">
    <a :href="docs_link" target="_blank" rel="noopener">{{ title }}</a>
    <p class="result" v-for="sentence in highlights">
      <template v-for="part in sentence">
        <span :class="{ highlighted: part.highlighted }">{{ part.text }}</span>
      </template>
    </p>
  </p>
</template>

<style scoped>

p.episode {
  font-size: 1.22rem;
}

a {
  font-family: "Roboto", sans-serif;
  font-weight: bold;
  font-size: 1.9rem;
  font-style: italic;
  letter-spacing: 3px;
  transition: 0.2s;
  margin-top: 100px;
}

a:hover {
  border-bottom: 3px solid #ffcc00;
  color: #ffcc00;
  transition: 0.1s;
}

p.episode p {
  border-bottom: 1px solid #684cb0;
  margin-bottom: 40px;
  padding-bottom: 5px;
  text-align: left;
}

.highlighted {
  color: #ffcc00;
  font-weight: bold;
}
</style>
