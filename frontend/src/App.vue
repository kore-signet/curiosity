<script setup lang="ts">
import * as api from './api'
import { ref, type Ref } from 'vue'
import Episode from './components/Episode.vue'
import Form from './components/Form.vue'
import type { ApiResponse, EpisodeData } from './types'

const episodes: Ref<EpisodeData[]> = ref([])
let page: string | null = null

function search_response(res: ApiResponse) {
  episodes.value = res.episodes
  page = res.next_page
}

async function load_more() {
  let res = await api.search({ page: page })
  page = res.next_page
  episodes.value = episodes.value.concat(res.episodes)
}

</script>

<template>
  <main>
    <h2>
      <span id="small-title">SEARCH AT THE<br /></span>
      <span id="large-title">TABLE</span>
    </h2>
    <p>
      An actual play web app that allows you to search for key terms across a
      whole season of Friends at the Table transcripts at once!
    </p>
    <p>
      Just select which season of FaTT you want to search through and enter
      your search term.<br>You can also click the episode title to go
      straight to the transcript itself. You can view all completed
      transcripts by
      <a id="transcripts-link"
        href="https://docs.google.com/spreadsheets/d/1KZHwlSBvHtWStN4vTxOTrpv4Dp9WQrulwMCRocXeYcQ/edit#gid=688483886">clicking
        here</a>
    </p>
    <Form @search="search_response" />
    <div class="output" aria-live="polite">
      <p style="margin: 2rem 0; font-size: 1.2rem;" v-show="episodes.length > 0">Total Results: <b>{{ episodes.length
      }}</b></p>
      <Episode v-for="episode in episodes" v-bind="episode" />
      <button id="load-more" @click="load_more" v-show="page">Load more</button>
    </div>
  </main>

  <footer>
    <p>
    <p>powered by
      <a href="transcriptsatthetable.com" class="link" target="_blank" rel="noopener">transcriptsatthetable.com</a><br />
    </p>
    originally by
    <a href="https://twitter.com/bryanbakedbean" class="link" target="_blank" rel="noopener">@bryanbakedbean</a>
    /
    currently upkept by
    <a class="link" href="https://twitter.com/sapphiclinguine" target="_blank" rel="noopener">emily signet
      (@sapphiclinguine)</a>
    <br />
    </p>
    <p>
      <a class="link" href="docs.html" target="_blank" rel="noopener">api docs</a>
      /
      <a class="link" href="https://github.com/emily-signet/curiosity" target="_blank" rel="noopener">source code</a>
      /
      <a class="link" href="https://memorious.cat-girl.gay" target="_blank" rel="noopener">library of memorious
        (backup/alternative)</a>
    </p>
  </footer>
</template>

<style scoped>
main {
  display: block;
  margin: 0 auto;
  width: 70%;
}

h2 {
  font-size: 2.8rem;
  font-family: "Roboto", sans-serif;
  letter-spacing: 7px;
  font-style: italic;
  line-height: 1.6em;
}

#small-title {
  font-weight: 400;
}

#large-title {
  font-weight: 700;
  font-size: 5.7rem;
}

#transcripts-link {
  border-bottom: 2px solid #ffcc00;
  color: #ffcc00;
}

#load-more {
  display: block;

  background: #684cb0;
  font-size: 1.3rem;
  color: white;
  border: none;
  transition: 0.2s;

  min-width: fit-content;
  min-height: 3rem;
  width: 7em;

  margin-bottom: 1em;
}

#load-more:hover {
  background: #ac57ff;
  transition: 0.2s;
  cursor: pointer;
}

.link {
  border-bottom: 2px solid #ffcc00;
  color: #ffcc00;
}

footer {
  margin-top: 7rem;
}
</style>
