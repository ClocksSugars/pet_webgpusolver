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
   -
