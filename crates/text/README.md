# Text

This implementation is based on the excellent work from [glyphon](https://github.com/grovesNL/glyphon). I would have loved to use glyphon directly; 
however, it is tightly coupled with wgpu for its rendering, and that isn't optional.

Given that we have our own gfx API, I had to "fork" and adapt the code to make it compatible.