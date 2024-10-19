#include <iostream>
#include <math.h>
#include <chrono>
#include <array>

// Kernel function to calculate densities
__global__
void calculate_densities(float **positions, float *densities, float *radii, int PARTICLE_AMOUNT) {
  int index = blockIdx.x * blockDim.x + threadIdx.x;
  if (index >= PARTICLE_AMOUNT) return;

  float density = 0.0;
  
  densities[index] = density;
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
    positions[i][1] = (i + 0.5) * (SCREEN_SIZE[1] - 2.0 *   PADDING) / PARTICLE_AMOUNT_Y + PADDING;
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

  // Get the number of blocks and threads
  int blockSize = 256;
  int numBlocks = (PARTICLE_AMOUNT + blockSize - 1) / blockSize;

  // Calculate densities
  calculate_densities<<<numBlocks, blockSize>>>(positions, densities, radii, PARTICLE_AMOUNT);

  // Wait for GPU to finish before accessing on host
  cudaDeviceSynchronize();

  // Print end time in ms
  auto end = std::chrono::high_resolution_clock::now();
  std::chrono::duration<double, std::milli> elapsed = end - start;
  std::cout << "Elapsed time in milliseconds : " << elapsed.count() << " ms" << std::endl;

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
