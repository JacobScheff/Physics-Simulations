#include <iostream>
#include <math.h>
#include <chrono>
#include <array>

// Kernel function to add the elements of two arrays
__global__
void add(int n, float *x, float *y)
{
  int index = blockIdx.x * blockDim.x + threadIdx.x;
  int stride = blockDim.x * gridDim.x;
  for (int i = index; i < n; i += stride)
    y[i] = x[i] + y[i];
}

int main(void)
{
  std::array<int, 2> const SCREEN_SIZE = {800, 400}; // The size of the screen
  int const TIME_BETWEEN_FRAMES = 2;
  std::array<int, 2> const GRID_SIZE = {80, 40}; // How many grid cells to divide the screen into

  float const PARTICLE_RADIUS = 1.25; // The radius of the particles
  int const PARTICLE_AMOUNT_X = 192; // The number of particles in the x direction
  int const PARTICLE_AMOUNT_Y = 96; // The number of particles in the y direction
  int const PARTICLE_AMOUNT = PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y; // The total number of particles
  float const PADDING = 50.0; // The padding around the screen

  // Initialize data
  float **positions, **velocities, **pressure_force, **viscosity_force;
  float *densities, *radii;

  // Allocate Unified Memory – accessible from CPU or GPU
  cudaMallocManaged(&positions, PARTICLE_AMOUNT * sizeof(float*));
  cudaMallocManaged(&velocities, PARTICLE_AMOUNT * sizeof(float*));
  cudaMallocManaged(&pressure_force, PARTICLE_AMOUNT * sizeof(float*));
  cudaMallocManaged(&viscosity_force, PARTICLE_AMOUNT * sizeof(float*));
  cudaMallocManaged(&densities, PARTICLE_AMOUNT * sizeof(float));
  cudaMallocManaged(&radii, PARTICLE_AMOUNT * sizeof(float));

  // Initialize data
  for (int i = 0; i < PARTICLE_AMOUNT; i++) {
    positions[i] = new float[2];
    velocities[i] = new float[2];
    pressure_force[i] = new float[2];
    viscosity_force[i] = new float[2];
    positions[i][0] = (i + 0.5) * (SCREEN_SIZE[0] - 2.0 * PADDING) / PARTICLE_AMOUNT_X + PADDING;
    positions[i][1] = (i + 0.5) * (SCREEN_SIZE[1] - 2.0 * PADDING) / PARTICLE_AMOUNT_Y + PADDING;
    velocities[i][0] = 0.0;
    velocities[i][1] = 0.0;
    densities[i] = 0.0;
    radii[i] = PARTICLE_RADIUS;
    pressure_force[i][0] = 0.0;
    pressure_force[i][1] = 0.0;
    viscosity_force[i][0] = 0.0;
    viscosity_force[i][1] = 0.0;
  }

  // Get start time
  auto start = std::chrono::high_resolution_clock::now();

  // // // Run kernel on the GPU
  // // int blockSize = 256;
  // // int numBlocks = (N + blockSize - 1) / blockSize;
  // // add<<<numBlocks, blockSize>>>(N, x, y);

  // Wait for GPU to finish before accessing on host
  cudaDeviceSynchronize();

  // Get end time
  auto end = std::chrono::high_resolution_clock::now();
  std::chrono::duration<float> duration = end - start;
  std::cout << "Time: " << duration.count() << "s" << std::endl;

  // Free memory
  cudaFree(positions);
  cudaFree(velocities);
  cudaFree(pressure_force);
  cudaFree(viscosity_force);
  cudaFree(densities);
  cudaFree(radii);

  std::cout << "Hello, World!" << std::endl;
  
  return 0;
}
