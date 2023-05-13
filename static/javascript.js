new TomSelect("#search_seasons", {
	plugins: {
		remove_button:{
			title:'Remove this item',
		},
    clear_button: {},
    no_active_items: {}
	}
});

// from https://github.com/sindresorhus/escape-string-regexp/blob/main/index.js
function escape_regex(s) {
  return s.replace(/[|\\{}()[\]^$+*?.]/g, "\\$&").replace(/-/g, "\\x2d");
}

function clearOutput() {
  document.getElementById("output").innerHTML = "";
}

//simple ajax function to send input to functions.php and display results as HTML within index.html
function searchText(show_total_results) {
  let season_dropdown = document.getElementById("search_seasons");
  let seasons = Array.from(season_dropdown.selectedOptions).map(opt => opt.value);
  if (seasons.indexOf("all-seasons") != -1) {
    seasons = [];
  }

  seasons = seasons.join(",");

  let query = document.getElementById("search_bar").value;

  let fmt_dropdown = document.getElementById("search_format");
  let query_fmt = fmt_dropdown.options[fmt_dropdown.selectedIndex].value;
  console.log(seasons);
  fetch_then_display({
    q: query,
    seasons: seasons,
    highlight: true,
    kind: query_fmt,
    // return_text: false,
    // return_highlights: true,
  }, true);
  //   .then((response) => response.json())
  //   .then((res) => {
  //     let episodes = res["data"]
  //       .sort((a, b) => {
  //         return a["title"].localeCompare(b["title"], undefined, {
  //           numeric: true,
  //           sensitivity: "base",
  //         });
  //       })
  //       .map((ep) => {
  //         let highlights;
  //         if (ep["highlights"]) {
  //           highlights = ep["highlights"]
  //             .map(
  //               (sentence) =>
  //                 `<p class="search_result">${sentence
  //                   .replace("<highlight>", '<span class="keyword_highlight">')
  //                   .replace("</highlight>", "</span>")}</p>`
  //             )
  //             .join("");
  //         } else {
  //           highlights = "";
  //         }


  //       })
  //       .join("");


  //   });
}

function fetch_then_display(params, show_total_results) {
  fetch(
    "/api/search?" +
      new URLSearchParams(params)
  ).then((response) => response.json())
  .then((res) => {
    let load_more = document.getElementById("load-more");
    if (res["next_page"]) {
      load_more.style = "";
      load_more.dataset.page = res["next_page"];
    } else {
      load_more.style = "display: none";
    }

    let episodes = res.episodes.map((ep) => {
      let highlights;
      if (ep["highlights"]) {
        highlights = ep["highlights"].map((sentence) => {
          sentence = sentence.map((part) => {
            if (part["highlighted"]) {
              return `<span class="keyword_highlight">${part["text"]}</span>`
            } else {
              return part["text"];
            }
          }).join("");
          return `<p class="search_result">${sentence}</p>`
        }).join("");
      } else {
        highlights = ""
      };

      return `
      <p>
          <a class="episode_title" href="https://docs.google.com/document/d/${ep["docs_id"]}" target="_blank" rel="noopener">${ep["title"]}</a>
      </p>
      ${highlights}
      `; 
    }).join("");

    document.getElementById("fountainG").style.display = "none";
    if (show_total_results) {
      document.getElementById("output").innerHTML += `<p class="total_result">Total Results: <span class="bold">${res["episodes"].length}</span></p>`

    }
    document.getElementById("output").innerHTML +=  episodes;
  })
}

function load_more() {
  fetch_then_display({page:document.getElementById("load-more").dataset.page}, false);
}