#include <iostream>
#include <math.h>
#include <chrono>
#include <array>
#include <vector>

std::array<int, 2> const SCREEN_SIZE = {800, 400}; // The size of the screen
std::array<int, 2> const GRID_SIZE = {80, 40};     // How many grid cells to divide the screen into

int const TIME_BETWEEN_FRAMES = 2;
float const PARTICLE_RADIUS = 1.25;                                // The radius of the particles
int const PARTICLE_AMOUNT_X = 192;                                 // The number of particles in the x direction
int const PARTICLE_AMOUNT_Y = 96;                                  // The number of particles in the y direction
int const PARTICLE_AMOUNT = PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y; // The total number of particles
float const PADDING = 50.0;                                        // The padding around the screen

// Grid functions
__host__
std::array<int, 2> pos_to_grid(float x, float y)
{
  return {
      (int)fmax(fmin(floor(x / SCREEN_SIZE[0] * GRID_SIZE[0]), GRID_SIZE[0] - 1), 0),
      (int)fmax(fmin(floor(y / SCREEN_SIZE[1] * GRID_SIZE[1]), GRID_SIZE[1] - 1), 0)};
}

__host__
int grid_to_index(int x, int y)
{
  return y * GRID_SIZE[0] + x;
}

// Kernel function to calculate densities
__global__ void calculate_densities(float **positions, float *densities, float *radii, int PARTICLE_AMOUNT)
{
  int index = blockIdx.x * blockDim.x + threadIdx.x;
  if (index >= PARTICLE_AMOUNT)
    return;

  float density = 0.0;

  // TODO: Finish code

  densities[index] = density;
}

void sort(float **positions, float **velocities, float *radii, float *densities, float **pressure_force, float **viscosity_force, int *particle_lookup, int *particle_counts)
{
  // Map all particles to their grid cell
  std::vector<std::vector<std::vector<int>>> index_map(GRID_SIZE[0], std::vector<std::vector<int>>(GRID_SIZE[1], std::vector<int>()));
  for (int i = 0; i < PARTICLE_AMOUNT; i++)
  {
    std::array<int, 2> grid = pos_to_grid(positions[i][0], positions[i][1]);
    index_map[grid[0]][grid[1]].push_back(i);
  }

  // Create a new list of particles
  float **new_positions = new float *[PARTICLE_AMOUNT];
  float **new_velocities = new float *[PARTICLE_AMOUNT];
  float *new_radii = new float[PARTICLE_AMOUNT];
  float *new_densities = new float[PARTICLE_AMOUNT];
  float **new_pressure_force = new float *[PARTICLE_AMOUNT];
  float **new_viscosity_force = new float *[PARTICLE_AMOUNT];

  // Iterate over all grid cells
  for (int i = 0; i < GRID_SIZE[0]; i++)
  {
    for (int j = 0; j < GRID_SIZE[1]; j++)
    {
      int grid_index = i + j * GRID_SIZE[0];
      int index = -1;

      // Iterate over all particles in the grid cell
      for (int k = 0; k < index_map[i][j].size(); k++)
      {
        int particle_index = index_map[i][j][k];
        new_positions[particle_index] = positions[particle_index];
        new_velocities[particle_index] = velocities[particle_index];
        new_radii[particle_index] = radii[particle_index];
        new_densities[particle_index] = densities[particle_index];
        new_pressure_force[particle_index] = pressure_force[particle_index];
        new_viscosity_force[particle_index] = viscosity_force[particle_index];

        if (index == -1)
        {
          index = particle_index;
        }
        particle_counts[grid_index]++;
      }

      particle_lookup[grid_index] = index;
    }
  }

  positions = new_positions;
  velocities = new_velocities;
  radii = new_radii;
  densities = new_densities;
  pressure_force = new_pressure_force;
  viscosity_force = new_viscosity_force;
}

int main(void)
{
  // Initialize data
  float **positions, **velocities, **pressure_force, **viscosity_force;
  float *densities, *radii;
  int *particle_lookup, *particle_counts;

  // Allocate Unified Memory – accessible from CPU or GPU
  cudaMallocManaged(&positions, PARTICLE_AMOUNT * sizeof(float *));
  cudaMallocManaged(&velocities, PARTICLE_AMOUNT * sizeof(float *));
  cudaMallocManaged(&pressure_force, PARTICLE_AMOUNT * sizeof(float *));
  cudaMallocManaged(&viscosity_force, PARTICLE_AMOUNT * sizeof(float *));
  cudaMallocManaged(&densities, PARTICLE_AMOUNT * sizeof(float));
  cudaMallocManaged(&radii, PARTICLE_AMOUNT * sizeof(float));
  cudaMallocManaged(&particle_lookup, GRID_SIZE[0] * GRID_SIZE[1] * sizeof(int));
  cudaMallocManaged(&particle_counts, GRID_SIZE[0] * GRID_SIZE[1] * sizeof(int));

  // Initialize data
  for (int i = 0; i < PARTICLE_AMOUNT; i++)
  {
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

    if (i < GRID_SIZE[0] * GRID_SIZE[1])
    {
      particle_lookup[i] = -1;
      particle_counts[i] = 0;
    }
  }
  
  // Sort the particles
  sort(positions, velocities, radii, densities, pressure_force, viscosity_force, particle_lookup, particle_counts);

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