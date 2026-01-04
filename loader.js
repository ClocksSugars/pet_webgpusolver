const canvas = document.getElementById("myCanvas");
import init from "./pkg/pet_webgpusolver.js";

async function run() {
   await init()
}

run();

console.log("WASM Loaded");

function validateintbox(element) {
   var val = Math.floor(element.value);
   if (val < 1) {val = 1}
   element.value = val;
}

document.getElementById("delta_x_y").addEventListener("change", (event) => {
   var val = Math.floor(
      1. / document.getElementById("delta_x_y").value
   );
   if (val < 1) {val = 1}
   let length_val = document.getElementById("length_val");
   length_val.value = val;

   document.getElementById("length_display").textContent = `${val}x`;
})

document.getElementById("length_val").addEventListener("change", (event) => {
   var val = Math.floor(
      document.getElementById("length_val").value
   );
   if (val < 1) {val = 1}
   let length_val = document.getElementById("length_val");
   document.getElementById("delta_x_y").value = 1.0 / val;

   document.getElementById("length_display").textContent = `${val}x`;
})

document.getElementById("max_N").addEventListener("change", (event) => {
   validateintbox(event.target);
})
document.getElementById("N_add").addEventListener("change", (event) => {
   validateintbox(event.target);
})



import {
   run_a_compute_iter,
   render_a_frame,
   update_values,
} from "./pkg/pet_webgpusolver.js";

var max_N = 52488;
var N_add = 100;
var current_N = 0;
var stop_compute = false;

function run_compute() {
   run_a_compute_iter();
   render_a_frame();

   current_N = current_N + N_add

   if ((current_N < max_N) & (~stop_compute)) {requestAnimationFrame(run_compute);}
}

document.getElementById("compute").addEventListener("click", (event) => {
   current_N = 0;
   stop_compute = false;
   requestAnimationFrame(run_compute);
})

document.getElementById("break").addEventListener("click", (event) => {
   stop_compute = true;
})

document.getElementById("update_vals").addEventListener("click", (event) => {
   update_values(
      document.getElementById("N_add").value,
      document.getElementById("kappa").value,
      document.getElementById("delta_t").value,
      document.getElementById("min_T").value,
      document.getElementById("max_T").value,
   )
})
