# the plan
## initial conditions
probably load in a csv. prior to this begin with a circle of some constant (set as a variable) temperature in the center. We work only on a unit square \[0,1\]x\[0.1\]. Set a spatial resolution parameter and a time resolution parameter, which will initially give us a 256x256 grid, and we add generality for that later.

## viewing data
ultimately we will want to be able to display solutions in a browser, but we'll need some way to view shader outputs while testing webgpu. at first we'll output to a png.

## the shader
there are a series of tests we must go through to make sure we are doing this right and minimize sources of error when we debug for each thing that will inevitably go wrong. these steps are:
   - make sure we can copy initial conditions in and out of buffers freely
   - make sure we can adjust buffers with the shaders in some trivial way, say by moving them all to the left.
   - make sure we can compute a laplacian

# we did that, new plan
## get a window open on web
this turns out to be mostly easy

## get an initial value showing on it
to do this we'll have to set up the hsv -> rgb shader. my plan is to have this be a compute shader and we'll copy it into the texture buffer when we change it, since we arent exactly aiming for crazy frame rates; this also means we dont have to worry about interpolation stuff when the canvas is stretched, texture buffer tooling handles this for us. the new to do list is:
   - write a heat map shader
   - write an initialization thing for the compute pipeline
   - try to crunch the code down a little with some abstractions
