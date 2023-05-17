import { reactive } from "vue";
import { QueryKind, type ApiRequest, type ApiResponse } from "./types";

export async function search(
    request: Partial<ApiRequest>
): Promise<ApiResponse> {
    let req: Record<string, string> = {
        query: request.query || "",
        kind: request.kind || QueryKind.PHRASE,
        seasons:  request.seasons ? request.seasons.join(",") : "",
        highlight: (request.highlight || true).toString(),
    };

    if (request.page) {
        req["page"] = request.page;
    }

    let res = await fetch(
        "http://localhost:8080/api/search?" + new URLSearchParams(req),
        { mode: "cors" }
    );
    return (await res.json()) as ApiResponse;
}

// why do i have to write this in the year of our lord 2023
export function array_unordered_equals(lhs: any[], rhs: any[]): boolean {
    lhs.sort();
    rhs.sort();

    if (lhs.length !== rhs.length) {
        return false;
    }

    for (let i = 0; i < lhs.length; i++) {
        if (lhs[i] !== rhs[i]) {
            return false;
        }
    }

    return true;
}

export const all_seasons = [
    "autumn-in-hieron",
    "marielda",
    "winter-in-hieron",
    "spring-in-hieron",
    "counterweight",
    "twilight-mirage",
    "road-to-partizan",
    "partizan",
    "road-to-palisade",
    "palisade",
    "sangfielle",
    "extras",
    "patreon",
    "other",
].sort();
