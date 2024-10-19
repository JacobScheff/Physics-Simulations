#include <iostream>
#include <math.h>
#include <chrono>
#include <vector>

#define SCREEN_SIZE_X 1200
#define SCREEN_SIZE_Y 600
#define GRID_SIZE_X 80
#define GRID_SIZE_Y 40
int const SCREEN_SIZE_C[2] = {SCREEN_SIZE_X, SCREEN_SIZE_Y}; // The size of the screen
int const GRID_SIZE_C[2] = {GRID_SIZE_X, GRID_SIZE_Y};     // How many grid cells to divide the screen into

int const TIME_BETWEEN_FRAMES = 2;
float const PARTICLE_RADIUS = 1.25;                                // The radius of the particles
int const PARTICLE_AMOUNT_X = 192;                                 // The number of particles in the x direction
int const PARTICLE_AMOUNT_Y = 96;                                  // The number of particles in the y direction
int const PARTICLE_AMOUNT = PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y; // The total number of particles
float const PADDING = 50.0;                                        // The padding around the screen

#define RADIUS_OF_INFLUENCE (75.0 / 4.0) // The radius of the sphere of influence. Also the radius to search for particles to calculate the density
#define TARGET_DENSITY 0.2; // The target density of the fluid
#define PRESSURE_MULTIPLIER 500.0; // The multiplier for the pressure force
#define GRAVITY 0.2; // The strength of gravity
#define LOOK_AHEAD_TIME (1.0 / 60.0); // The time to look ahead when calculating the predicted position
#define VISCOSITY 0.1; // The viscosity of the fluid
#define DAMPENING 0.95; // How much to slow down particles when they collide with the walls
#define dt (1.0 / 8.0); // The time step

int const GRIDS_TO_CHECK[2] = {int(RADIUS_OF_INFLUENCE / SCREEN_SIZE_C[0] * GRID_SIZE_C[0] + 1.0), int(RADIUS_OF_INFLUENCE / SCREEN_SIZE_C[1] * GRID_SIZE_C[1] + 1.0)}; // How many grid cells to check in each direction

// Grid functions
__device__ __host__
int* pos_to_grid(float x, float y)
{
  int grid[2] =  {
      (int)fmax(fmin(floor(x / SCREEN_SIZE_X * GRID_SIZE_X), GRID_SIZE_X - 1), 0),
      (int)fmax(fmin(floor(y / SCREEN_SIZE_Y * GRID_SIZE_Y), GRID_SIZE_Y - 1), 0)};
  return grid;
}

__device__ __host__
int grid_to_index(int x, int y)
{
  return y * GRID_SIZE_X + x;
}

// Kernel function to calculate densities
__global__ void calculate_densities(float **positions, float *densities, float *radii, int *particle_lookup, int *particle_counts, int GRIDS_TO_CHECK_X, int GRIDS_TO_CHECK_Y, int particle_amount)
{
  int index = blockIdx.x * blockDim.x + threadIdx.x;
  if (index >= PARTICLE_AMOUNT)
    return;

  int* grid = pos_to_grid(positions[index][0], positions[index][1]);
  float density = 0.0;

  for(int g = 0; g < (GRIDS_TO_CHECK_X * 2 + 1) * (GRIDS_TO_CHECK_Y * 2 + 1); g++){
    int gx = g / (GRIDS_TO_CHECK_Y * 2 + 1) - GRIDS_TO_CHECK_X;
    int gy = g % (GRIDS_TO_CHECK_Y * 2 + 1) - GRIDS_TO_CHECK_Y;

    if(grid[0] + gx < 0 || grid[0] + gx >= GRID_SIZE_X || grid[1] + gy < 0 || grid[1] + gy >= GRID_SIZE_Y){
      continue;
    }

    int first_grid_index = grid_to_index(grid[0] + gx, grid[1] + gy);
    if(first_grid_index < 0 || first_grid_index >= GRID_SIZE_X * GRID_SIZE_Y){
      continue;
    }

    int starting_index = particle_lookup[first_grid_index];
    if(starting_index == -1){
      continue;
    }

    int ending_index = starting_index + particle_counts[first_grid_index];

    for(int i = starting_index; i <= ending_index; i++){
      
    }
  }

  densities[index] = density;
}

void sort(float **positions, float **velocities, float *radii, float *densities, float **pressure_force, float **viscosity_force, int *particle_lookup, int *particle_counts)
{
  // Map all particles to their grid cell
  std::vector<std::vector<std::vector<int>>> index_map(GRID_SIZE_C[0], std::vector<std::vector<int>>(GRID_SIZE_C[1], std::vector<int>()));
  for (int i = 0; i < PARTICLE_AMOUNT; i++)
  {
    int* grid = pos_to_grid(positions[i][0], positions[i][1]);
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
  for (int i = 0; i < GRID_SIZE_C[0]; i++)
  {
    for (int j = 0; j < GRID_SIZE_C[1]; j++)
    {
      int grid_index = i + j * GRID_SIZE_C[0];
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

  delete[] new_positions;
  delete[] new_velocities;
  delete[] new_radii;
  delete[] new_densities;
  delete[] new_pressure_force;
  delete[] new_viscosity_force;
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
  cudaMallocManaged(&particle_lookup, GRID_SIZE_C[0] * GRID_SIZE_C[1] * sizeof(int));
  cudaMallocManaged(&particle_counts, GRID_SIZE_C[0] * GRID_SIZE_C[1] * sizeof(int));

  // Initialize data
  for (int i = 0; i < PARTICLE_AMOUNT; i++)
  {
    positions[i] = new float[2];
    velocities[i] = new float[2];
    pressure_force[i] = new float[2];
    viscosity_force[i] = new float[2];
    positions[i][0] = (i + 0.5) * (SCREEN_SIZE_C[0] - 2.0 * PADDING) / PARTICLE_AMOUNT_X + PADDING;
    positions[i][1] = (i + 0.5) * (SCREEN_SIZE_C[1] - 2.0 * PADDING) / PARTICLE_AMOUNT_Y + PADDING;
    velocities[i][0] = 0.0;
    velocities[i][1] = 0.0;
    densities[i] = 0.0;
    radii[i] = PARTICLE_RADIUS;
    pressure_force[i][0] = 0.0;
    pressure_force[i][1] = 0.0;
    viscosity_force[i][0] = 0.0;
    viscosity_force[i][1] = 0.0;

    if (i < GRID_SIZE_C[0] * GRID_SIZE_C[1])
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
  calculate_densities<<<numBlocks, blockSize>>>(positions, densities, radii, particle_lookup, particle_counts, GRIDS_TO_CHECK[0], GRIDS_TO_CHECK[1], PARTICLE_AMOUNT);

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
  cudaFree(particle_lookup);
  cudaFree(particle_counts);

  std::cout << "Hello, World!" << std::endl;

  return 0;
}

__device__
float density_to_pressure(float density)
{
  float density_error = density - TARGET_DENSITY;
  return density_error * PRESSURE_MULTIPLIER;
}

__device__
float smoothing_kernel(float distance)
{
  if (distance >= RADIUS_OF_INFLUENCE)
  {
    return 0.0;
  }

  float volume = 3.141592653589 * pow(RADIUS_OF_INFLUENCE, 4.0) / 6.0;
  return (RADIUS_OF_INFLUENCE - distance) * (RADIUS_OF_INFLUENCE - distance) / volume;
}

__device__
float smoothing_kernel_derivative(float distance)
{
  if (distance >= RADIUS_OF_INFLUENCE)
  {
    return 0.0;
  }

  float scale = 12.0 / (pow(RADIUS_OF_INFLUENCE, 4.0) * 3.141592653589);
  return (RADIUS_OF_INFLUENCE - distance) * scale;
}

__device__
float viscosity_kernel(float distance)
{
  if (distance >= RADIUS_OF_INFLUENCE)
  {
    return 0.0;
  }

  float volume = 3.141592653589 * pow(RADIUS_OF_INFLUENCE, 8.0) / 4.0;
  float value = RADIUS_OF_INFLUENCE * RADIUS_OF_INFLUENCE - distance * distance;
  return value * value * value / volume;
}

__device__
float calculate_shared_pressure(float density_a, float density_b)
{
  float pressure_a = density_to_pressure(density_a);
  float pressure_b = density_to_pressure(density_b);
  return (pressure_a + pressure_b) / 2.0;
}