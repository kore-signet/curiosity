export type EpisodeData = {
    curiosity_id: number,
    slug: string,
    season: string,
    title: string,
    docs_id: string,
    highlights: { text: string, highlighted: boolean }[][]
}

export enum QueryKind  {
    KEYWORDS = "keywords",
    PHRASE = "phrase",
    WEBSEARCH = "web"
}

export interface ApiRequest  {
    query: string,
    kind: QueryKind,
    seasons?: string[],
    highlight: boolean,
    page?: string | null,
    page_size?: number
}

export type ApiResponse = {
    next_page: string | null,
    episodes: EpisodeData[]
}