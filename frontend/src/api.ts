import { reactive } from "vue";
import { QueryKind, type ApiRequest, type ApiResponse } from "./types";

export async function search(request: Partial<ApiRequest>): Promise<ApiResponse> {
    let req: Record<string, string> = {
        query: request.query || "",
        kind: request.kind || QueryKind.PHRASE,
        seasons: request.seasons ? request.seasons.join(",") : "",
        highlight: (request.highlight || true).toString()
    }

    if (request.page) {
        req["page"] = request.page
    }

    let res = await fetch("/api/search?" + new URLSearchParams(req), { mode: "cors"})
    return (await res.json()) as ApiResponse
}