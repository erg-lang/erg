let sections = document.querySelectorAll('section');
let searchBox = document.getElementById('search-box');
function searchAndFilter() {
    let query = searchBox.value;
    for (const section of sections) {
        if (section.textContent.toLowerCase().includes(query.toLowerCase())) {
            section.style.display = "";
        } else {
            section.style.display = "none";
        }
    }
}

let timeoutId;
searchBox.addEventListener('keyup', () => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(searchAndFilter);
});
