"use strict";

document.addEventListener("DOMContentLoaded", function(event) {
        Array.from(document.getElementsByClassName("time")).forEach(item => {
                let ts = item.getAttribute("data-time");
                let date = new Date(ts * 1000);

                item.innerHTML = `${date.toLocaleDateString("nl-be")} ${date.toLocaleTimeString("nl-be")}`;
        })
});

