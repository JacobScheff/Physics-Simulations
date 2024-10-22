#include <iostream>
#include <math.h>
#include <chrono>
#include <vector>
#include <algorithm>
#include <SFML/Graphics.hpp>

// nvcc main.cu -I"C:\\Users\\jacob\\Documents\\VSC\\C++ Libraries\\SFML-2.6.1\\include" -L"C:\\Users\\jacob\\Documents\\VSC\\C++ Libraries\\SFML-2.6.1\\lib" -lsfml-graphics -lsfml-window -lsfml-system && a.exe

#define SCREEN_SIZE_X 1200
#define SCREEN_SIZE_Y 600
#define GRID_SIZE_X 80
#define GRID_SIZE_Y 40

int const TIME_BETWEEN_FRAMES = 2;                                 // The time between frames in milliseconds
float const PARTICLE_RADIUS = 1.25;                                // The radius of the particles
int const PARTICLE_AMOUNT_X = 192;                                 // The number of particles in the x direction
int const PARTICLE_AMOUNT_Y = 96;                                  // The number of particles in the y direction
int const PARTICLE_AMOUNT = PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y; // The total number of particles
float const PADDING = 50.0;                                        // The padding around the screen

#define RADIUS_OF_INFLUENCE (75.0 / 4.0) // The radius of the sphere of influence. Also the radius to search for particles to calculate the density
#define TARGET_DENSITY 0.2;              // The target density of the fluid
#define PRESSURE_MULTIPLIER 500.0;       // The multiplier for the pressure force
#define GRAVITY 0.2;                     // The strength of gravity
// TODO: ADD BACK // #define LOOK_AHEAD_TIME (1.0 / 60.0); // The time to look ahead when calculating the predicted position
#define VISCOSITY 0.1;  // The viscosity of the fluid
#define DAMPENING 0.95; // How much to slow down particles when they collide with the walls
#define dt (1.0 / 8.0); // The time step

int const GRIDS_TO_CHECK[2] = {int(RADIUS_OF_INFLUENCE / SCREEN_SIZE_X * GRID_SIZE_X + 1.0), int(RADIUS_OF_INFLUENCE / SCREEN_SIZE_Y * GRID_SIZE_Y + 1.0)}; // How many grid cells to check in each direction

struct Particle
{
  sf::Vector2f position;
  sf::Vector2f velocity = {0.0, 0.0};
  float radius = PARTICLE_RADIUS;
  float density = 0.0;
  sf::Vector2f pressure_force = {0.0, 0.0};
  sf::Vector2f viscosity_force = {0.0, 0.0};
  int grid_index = -1;
};

// Grid functions
__host__ int *pos_to_grid(float x, float y)
{
  static int grid[2];
  grid[0] = (int)fmax(fmin(floor(x / SCREEN_SIZE_X * GRID_SIZE_X), GRID_SIZE_X - 1), 0);
  grid[1] = (int)fmax(fmin(floor(y / SCREEN_SIZE_Y * GRID_SIZE_Y), GRID_SIZE_Y - 1), 0);
  return grid;
}

__device__ void pos_to_grid(float x, float y, int grid[2])
{ // Pass grid as an argument
  grid[0] = (int)max(min((int)floor(x / SCREEN_SIZE_X * GRID_SIZE_X), GRID_SIZE_X - 1), 0);
  grid[1] = (int)max(min((int)floor(y / SCREEN_SIZE_Y * GRID_SIZE_Y), GRID_SIZE_Y - 1), 0);
}

__device__ __host__ int grid_to_index(int x, int y)
{
  return y * GRID_SIZE_X + x;
}

__device__ float density_to_pressure(float density)
{
  float density_error = density - TARGET_DENSITY;
  return density_error * PRESSURE_MULTIPLIER;
}

__device__ float smoothing_kernel(float distance)
{
  if (distance >= RADIUS_OF_INFLUENCE)
  {
    return 0.0;
  }

  float volume = 3.141592653589 * pow(RADIUS_OF_INFLUENCE, 4.0) / 6.0;
  return (RADIUS_OF_INFLUENCE - distance) * (RADIUS_OF_INFLUENCE - distance) / volume;
}

__device__ float smoothing_kernel_derivative(float distance)
{
  if (distance >= RADIUS_OF_INFLUENCE)
  {
    return 0.0;
  }

  float scale = 12.0 / (pow(RADIUS_OF_INFLUENCE, 4.0) * 3.141592653589);
  return (RADIUS_OF_INFLUENCE - distance) * scale;
}

__device__ float viscosity_kernel(float distance)
{
  if (distance >= RADIUS_OF_INFLUENCE)
  {
    return 0.0;
  }

  float volume = 3.141592653589 * pow(RADIUS_OF_INFLUENCE, 8.0) / 4.0;
  float value = RADIUS_OF_INFLUENCE * RADIUS_OF_INFLUENCE - distance * distance;
  return value * value * value / volume;
}

__device__ float calculate_shared_pressure(float density_a, float density_b)
{
  float pressure_a = density_to_pressure(density_a);
  float pressure_b = density_to_pressure(density_b);
  return (pressure_a + pressure_b) / 2.0;
}

// Kernel function to calculate densities
__global__ void calculate_densities(Particle *particles, int *particle_lookup, int *particle_counts, int GRIDS_TO_CHECK_X, int GRIDS_TO_CHECK_Y, int particle_amount)
{
  int index = blockIdx.x * blockDim.x + threadIdx.x;
  if (index >= PARTICLE_AMOUNT)
    return;

  int grid[2];
  pos_to_grid(particles[index].position.x, particles[index].position.y, grid);
  float density = 0.0;

  for (int g = 0; g < (GRIDS_TO_CHECK_X * 2 + 1) * (GRIDS_TO_CHECK_Y * 2 + 1); g++)
  {
    int gx = g / (GRIDS_TO_CHECK_Y * 2 + 1) - GRIDS_TO_CHECK_X;
    int gy = g % (GRIDS_TO_CHECK_Y * 2 + 1) - GRIDS_TO_CHECK_Y;

    if (grid[0] + gx < 0 || grid[0] + gx >= GRID_SIZE_X || grid[1] + gy < 0 || grid[1] + gy >= GRID_SIZE_Y)
    {
      continue;
    }

    int first_grid_index = grid_to_index(grid[0] + gx, grid[1] + gy);
    if (first_grid_index < 0 || first_grid_index >= GRID_SIZE_X * GRID_SIZE_Y)
    {
      continue;
    }

    int starting_index = particle_lookup[first_grid_index];
    if (starting_index == -1)
    {
      continue;
    }

    int ending_index = starting_index + particle_counts[first_grid_index] - 1;
    if (ending_index >= PARTICLE_AMOUNT)
    {
      ending_index = PARTICLE_AMOUNT - 1;
    }

    float x = particles[index].position.x;
    float y = particles[index].position.y;
    for (int i = starting_index; i <= ending_index; i++)
    {
      float distance = sqrtf((particles[i].position.x - x, 2.0) * (particles[i].position.x - x, 2.0) + (particles[i].position.y - y, 2.0) * (particles[i].position.y - y, 2.0));
      if (distance < RADIUS_OF_INFLUENCE)
      {
        float influence = smoothing_kernel(distance);
        density += influence * 3.1415926f * particles[i].radius * particles[i].radius;
      }
    }
  }

  particles[index].density = density;
}

// Kernel function to calculate forces
__global__ void calculate_forces(Particle *particles, int *particle_lookup, int *particle_counts, int GRIDS_TO_CHECK_X, int GRIDS_TO_CHECK_Y, int particle_amount)
{
  int index = blockIdx.x * blockDim.x + threadIdx.x;
  if (index >= PARTICLE_AMOUNT)
    return;

    int grid[2];
    pos_to_grid(particles[index].position.x, particles[index].position.y, grid);

    float pressure_force[2] = {0.0, 0.0};
    float viscosity_force[2] = {0.0, 0.0};

  for (int g = 0; g < (GRIDS_TO_CHECK_X * 2 + 1) * (GRIDS_TO_CHECK_Y * 2 + 1); g++)
  {
    int gx = g / (GRIDS_TO_CHECK_Y * 2 + 1) - GRIDS_TO_CHECK_X;
    int gy = g % (GRIDS_TO_CHECK_Y * 2 + 1) - GRIDS_TO_CHECK_Y;

    if (grid[0] + gx < 0 || grid[0] + gx >= GRID_SIZE_X || grid[1] + gy < 0 || grid[1] + gy >= GRID_SIZE_Y)
    {
      continue;
    }

    int first_grid_index = grid_to_index(grid[0] + gx, grid[1] + gy);
    if (first_grid_index < 0 || first_grid_index >= GRID_SIZE_X * GRID_SIZE_Y)
    {
      continue;
    }

    int starting_index = particle_lookup[first_grid_index];
    if (starting_index == -1)
    {
      continue;
    }

    int ending_index = starting_index + particle_counts[first_grid_index];

    for (int i = starting_index; i <= ending_index; i++)
    {
          // float offset[2] = {particles[i].position.x - particles[index].position.x, particles[i].position.y - particles[index].position.y};
      // float distance = sqrt(offset.x * offset.x + offset.y * offset.y);
  //     // if (distance == 0 || distance >= RADIUS_OF_INFLUENCE)
  //     // {
  //     //   continue;
  //     // }
  //     // sf::Vector2f dir = offset / distance;

  //     // float slope = smoothing_kernel_derivative(distance);
  //     // float shared_pressure = calculate_shared_pressure(particles[index].density, particles[i].density);

  //     // float pressure_multiplier = shared_pressure * slope * 3.141592653589 * particles[i].radius * particles[i].radius / max(particles[index].density, 0.000001);
  //     // sf::Vector2f local_pressure_force = dir * pressure_multiplier;

  //     // sf::Vector2f local_viscosity_force = (particles[i].velocity - particles[index].velocity) * viscosity_kernel(distance);
  //     // local_viscosity_force.x *= VISCOSITY;
  //     // local_viscosity_force.y *= VISCOSITY;

  //     // pressure_force += local_pressure_force;
  //     // viscosity_force += local_viscosity_force;
    }
  }

  particles[index].pressure_force.x = pressure_force[0];
  particles[index].pressure_force.y = pressure_force[1];
  particles[index].viscosity_force.x = viscosity_force[0];
  particles[index].viscosity_force.y = viscosity_force[1];
}

void sort(std::vector<Particle> &particles, std::vector<int> &particle_lookup, std::vector<int> &particle_counts)
{
  // Update the grid indices of the particles
  for (int i = 0; i < PARTICLE_AMOUNT; i++)
  {
    int *grid = pos_to_grid(particles[i].position.x, particles[i].position.y);
    particles[i].grid_index = grid_to_index(grid[0], grid[1]);
  }

  // Sort the particles based on grid index
  std::sort(particles.begin(), particles.end(), [](const Particle &a, const Particle &b)
            { return a.grid_index < b.grid_index; });

  // Update the particle lookup and counts
  for (int i = 0; i < GRID_SIZE_X * GRID_SIZE_Y; i++)
  {
    particle_lookup[i] = -1;
    particle_counts[i] = 0;
  }

  for (int i = 0; i < PARTICLE_AMOUNT; i++)
  {
    if (particle_lookup[particles[i].grid_index] == -1)
    {
      particle_lookup[particles[i].grid_index] = i;
    }
    particle_counts[particles[i].grid_index]++;
  }

  int currentGridIndex = -1;
  for (int i = 0; i < PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y; ++i)
  {
    if (particles[i].grid_index != currentGridIndex)
    {
      particle_lookup[particles[i].grid_index] = i;
      currentGridIndex = particles[i].grid_index;
    }
    particle_counts[particles[i].grid_index]++;
  }
}

int main(void)
{
  sf::RenderWindow window(sf::VideoMode(SCREEN_SIZE_X, SCREEN_SIZE_Y), "Fluid Simulation");

  // Initialize data
  std::vector<Particle> particles(PARTICLE_AMOUNT);
  std::vector<int> particle_lookup(GRID_SIZE_X * GRID_SIZE_Y, -1);
  std::vector<int> particle_counts(GRID_SIZE_X * GRID_SIZE_Y, 0);

  for (int i = 0; i < PARTICLE_AMOUNT_X; i++)
  {
    for (int j = 0; j < PARTICLE_AMOUNT_Y; j++)
    {
      int index = i + j * PARTICLE_AMOUNT_X;
      particles[index].position = {(i + 0.5f) * (SCREEN_SIZE_X - 2.0f * PADDING) / PARTICLE_AMOUNT_X + PADDING, (j + 0.5f) * (SCREEN_SIZE_Y - 2.0f * PADDING) / PARTICLE_AMOUNT_Y + PADDING};
    }
  }

  for (int i = 0; i < GRID_SIZE_X * GRID_SIZE_Y; i++)
  {
    particle_lookup[i] = -1;
    particle_counts[i] = 0;
  }

  // Allocate Unified Memory – accessible from CPU or GPU
  Particle *d_particles;
  int *d_particle_lookup;
  int *d_particle_counts;
  cudaMalloc(&d_particles, PARTICLE_AMOUNT * sizeof(Particle));
  cudaMalloc(&d_particle_lookup, GRID_SIZE_X * GRID_SIZE_Y * sizeof(int));
  cudaMalloc(&d_particle_counts, GRID_SIZE_X * GRID_SIZE_Y * sizeof(int));

  // Sort the particles
  sort(particles, particle_lookup, particle_counts);

  while (window.isOpen())
  {
    sf::Event event;
    while (window.pollEvent(event))
    {
      if (event.type == sf::Event::Closed)
        window.close();
    }

    // Get start time
    auto start = std::chrono::high_resolution_clock::now();

    // Copy data to the GPU
    cudaMemcpy(d_particles, particles.data(), PARTICLE_AMOUNT * sizeof(Particle), cudaMemcpyHostToDevice);
    cudaMemcpy(d_particle_lookup, particle_lookup.data(), GRID_SIZE_X * GRID_SIZE_Y * sizeof(int), cudaMemcpyHostToDevice);
    cudaMemcpy(d_particle_counts, particle_counts.data(), GRID_SIZE_X * GRID_SIZE_Y * sizeof(int), cudaMemcpyHostToDevice);

    // Get the number of blocks and threads
    int blockSize = 256;
    int numBlocks = (PARTICLE_AMOUNT + blockSize - 1) / blockSize;

    // Calculate densities
    calculate_densities<<<numBlocks, blockSize>>>(d_particles, d_particle_lookup, d_particle_counts, GRIDS_TO_CHECK[0], GRIDS_TO_CHECK[1], PARTICLE_AMOUNT);

    // Wait for GPU to finish before accessing on host
    cudaDeviceSynchronize();
    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess)
    {
      std::cerr << "CUDA error: " << cudaGetErrorString(err) << std::endl;
    }

    // // Calculate forces
    // calculate_forces<<<numBlocks, blockSize>>>(d_particles, d_particle_lookup, d_particle_counts, GRIDS_TO_CHECK[0], GRIDS_TO_CHECK[1], PARTICLE_AMOUNT);
    
    // // Wait for GPU to finish before accessing on host
    // cudaDeviceSynchronize();
    // err = cudaGetLastError();
    // if (err != cudaSuccess)
    // {
    //   std::cerr << "CUDA error: " << cudaGetErrorString(err) << std::endl;
    // }

    // Copy data back from GPU
    cudaMemcpy(particles.data(), d_particles, PARTICLE_AMOUNT * sizeof(Particle), cudaMemcpyDeviceToHost);

    // Print end time in ms
    auto end = std::chrono::high_resolution_clock::now();
    std::chrono::duration<double, std::milli> elapsed = end - start;
    std::cout << "Elapsed time in milliseconds : " << elapsed.count() << " ms" << std::endl;

    // NOTE: DRAWING IS VERY SLOW
    std::vector<sf::CircleShape> circles;
    for (int i = 0; i < PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y; ++i)
    {
      sf::CircleShape circle(PARTICLE_RADIUS);
      circle.setFillColor(sf::Color::Blue);
      circle.setPosition(particles[i].position);
      circles.push_back(circle);
    }

    window.clear();
    for (const auto &circle : circles)
    {
      window.draw(circle);
    }
    window.display();

    // Wait for TIME_BETWEEN_FRAMES
    sf::sleep(sf::milliseconds(TIME_BETWEEN_FRAMES));
  }

  // Free memory
  cudaFree(d_particles);
  cudaFree(d_particle_lookup);
  cudaFree(d_particle_counts);

  std::cout << "Hello, World!" << std::endl;

  return 0;
}