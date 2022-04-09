// from https://github.com/sindresorhus/escape-string-regexp/blob/main/index.js
function escape_regex(s) {
  return s.replace(/[|\\{}()[\]^$+*?.]/g, "\\$&").replace(/-/g, "\\x2d");
}

//simple ajax function to send input to functions.php and display results as HTML within index.html
function searchText() {
  let dropdown = document.getElementById("search_type");
  let season = dropdown.options[dropdown.selectedIndex].value;
  let query = document.getElementById("search_bar").value;

  let fmt_dropdown = document.getElementById("search_format");
  let query_fmt = fmt_dropdown.options[fmt_dropdown.selectedIndex].value;

  fetch(
    "/api/search?" +
      new URLSearchParams({
        q: query,
        season: season,
        query_fmt: query_fmt,
        return_text: false,
        return_highlights: true,
      })
  )
    .then((response) => response.json())
    .then((res) => {
      let episodes = res["data"]
        .sort((a, b) => {
          return a["title"].localeCompare(b["title"], undefined, {
            numeric: true,
            sensitivity: "base",
          });
        })
        .map((ep) => {
          let highlights;
          if (ep["highlights"]) {
            highlights = ep["highlights"]
              .map(
                (sentence) =>
                  `<p class="search_result">${sentence
                    .replace("<highlight>", '<span class="keyword_highlight">')
                    .replace("</highlight>", "</span>")}</p>`
              )
              .join("");
          } else {
            highlights = "";
          }

          return `
                    <p>
                        <a class="episode_title" href="https://docs.google.com/document/d/${ep["id"]}" target="_blank" rel="noopener">${ep["title"]}</a>
                    </p>
                    ${highlights}
                `;
        })
        .join("");

      document.getElementById("fountainG").style.display = "none";
      document.getElementById("output").innerHTML = `
            <p class="total_result">Total Results: <span class="bold">${res["data"].length}</span></p>
            ${episodes}
    `;
    });
}
