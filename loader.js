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

document.getElementById("delta_x").addEventListener("change", (event) => {
   var val = Math.floor(
      1. / document.getElementById("delta_x").value
   );
   if (val < 1) {val = 1}
   document.getElementById("width_val").value = val;
})

document.getElementById("delta_y").addEventListener("change", (event) => {
   var val = Math.floor(
      1. / document.getElementById("delta_y").value
   );
   if (val < 1) {val = 1}
   document.getElementById("height_val").value = val;
})

document.getElementById("width_val").addEventListener("change", (event) => {
   var val = Math.floor(
      document.getElementById("width_val").value
   );
   if (val < 1) {val = 1}
   let length_val = document.getElementById("length_val");
   document.getElementById("delta_x").value = 1.0 / val;
})

document.getElementById("height_val").addEventListener("change", (event) => {
   var val = Math.floor(
      document.getElementById("height_val").value
   );
   if (val < 1) {val = 1}
   let length_val = document.getElementById("height_val");
   document.getElementById("delta_y").value = 1.0 / val;
})

document.getElementById("max_N").addEventListener("change", (event) => {
   validateintbox(event.target);
})
document.getElementById("N_add").addEventListener("change", (event) => {
   validateintbox(event.target);
})

document.getElementById("auto_delta_t").addEventListener("click", (event) => {
   var width = document.getElementById("width_val").value;
   var height = document.getElementById("height_val").value;
   var kappa = document.getElementById("kappa").value;
   var safety_factor = document.getElementById("kappa").value;
   document.getElementById("delta_t").value = safety_factor / (2 * kappa * (width ** 2 + height ** 2))
})


import {
   run_a_compute_iter,
   render_a_frame,
   update_values,
   send_output_to_export,
   //setup_temp_receiver,
   is_receiver_ready,
   get_export_to_num,
   junk_current_state,
   rinit_with_xy,
   init_with_csv,
   give_current_width,
   give_current_height
} from "./pkg/pet_webgpusolver.js";

var max_N = 52488;
var N_add = 100;
var current_N = 0;
var stop_compute = false;
var get_total_temp = true;

function run_compute() {
   run_a_compute_iter();
   if (get_total_temp) {
      send_output_to_export();
      render_a_frame();
      //setup_temp_receiver();
      get_total_temp = false;
   } else {
      render_a_frame();
   }

   current_N = current_N + N_add

   let receiver_response = is_receiver_ready();
   if (receiver_response) {
      //console.log(`tried to export num with receiver ${receiver_response}`)
      let total_energy = get_export_to_num();
      document.getElementById("message_receiver").textContent = `total energy: ${total_energy}\ntime: ${current_N}`
      get_total_temp = true;
   }

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
   max_N = document.getElementById("max_N").value
   update_values(
      document.getElementById("N_add").value,
      document.getElementById("kappa").value,
      document.getElementById("delta_t").value,
      document.getElementById("min_T").value,
      document.getElementById("max_T").value,
   )
})

async function reset_state_with_dims() {
   stop_compute = true;
   junk_current_state();
   var width = document.getElementById("width_val").value;
   var height = document.getElementById("height_val").value;
   await rinit_with_xy(width, height);
   current_N = 0;
   max_N = document.getElementById("max_N").value;
   update_values(
      document.getElementById("N_add").value,
      document.getElementById("kappa").value,
      document.getElementById("delta_t").value,
      document.getElementById("min_T").value,
      document.getElementById("max_T").value,
   );
   get_total_temp = true;
   current_grid_shape.textContent = `${width}x${height}`;
}

function showMessage(thestring) {
   document.getElementById("message_receiver").textContent = thestring;
}

document.getElementById("send_xy").addEventListener("click", (event) => {
   reset_state_with_dims();
})

document.getElementById("send_csv_to_gpu").addEventListener("click", (event) => {
   file = document.getElementById("take_in_csv").files[0];
   if (!file) {
       return;
     }
   if (!file.type.startsWith("csv")) {
       showMessage("Unsupported file type. Please select a text file.", "error");
       return;
     }
   const reader = new FileReader();
   reader.onload = () => {
      stop_compute = true;
      var result = init_with_csv(reader.result.readAsText());
      if (result.startsWith("success!")) {
         document.getElementById("width_val").value = give_current_width();
         document.getElementById("height_val").value = give_current_height();
      } else {
         showMessage(result);
      };
     };
})
