import './assets/main.css'
import "primevue/resources/primevue.min.css"
import "./assets/prime-theme.css";    
import PrimeVue from 'primevue/config'

import { createApp } from 'vue'
import App from './App.vue'
import VueSelect from "vue-select"


let app = createApp(App)
app.config.globalProperties.window = window

app.component("v-select", VueSelect)
    .use(PrimeVue)
    .mount('#app')