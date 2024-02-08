import pygame
import sys
import time
import math

screenSize = (1200, 600)
ballRadius = 8
fps = 100
gravity = 160
borderCollisionMultiplier = 0.9

class Particle(pygame.sprite.Sprite):
    # position and velocity is pygame.Vector2
    def __init__(self, position, velocity, radius):
        super().__init__()
        self.position = position
        self.velocity = velocity
        self.radius = radius
        self.color = (255, 255, 255)
        self.image = pygame.Surface((radius*2, radius*2), pygame.SRCALPHA)
        self.rect = self.image.get_rect(center=position)
        pygame.draw.circle(self.image, self.color, (radius, radius), radius)

    def update(self, dt):
        self.position += self.velocity * dt
        self.velocity.y += gravity * dt
        self.rect.center = self.position
        if self.position.x - self.radius < 0:
            self.position.x = self.radius
            self.velocity.x *= -borderCollisionMultiplier
        if self.position.x + self.radius > screenSize[0]:
            self.position.x = screenSize[0] - self.radius
            self.velocity.x *= -borderCollisionMultiplier
        if self.position.y - self.radius < 0:
            self.position.y = self.radius
            self.velocity.y *= -borderCollisionMultiplier
        if self.position.y + self.radius > screenSize[1]:
            self.position.y = screenSize[1] - self.radius
            self.velocity.y *= -borderCollisionMultiplier

particles = []
for i in range(10):
    particles.append(Particle(pygame.Vector2(100 * i, 25 * i), pygame.Vector2(100, 100), ballRadius))

pygame.init()
screen = pygame.display.set_mode(screenSize)
clock = pygame.time.Clock()

while True:
    dt = clock.tick(fps) / 1000
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            pygame.quit()
            sys.exit()

    for particle in particles:
        particle.update(dt)

    # Check for collision
    for i in range(len(particles)):
        for j in range(i+1, len(particles)):
            distance = particles[i].position.distance_to(particles[j].position)
            if distance < particles[i].radius + particles[j].radius:
                normal = particles[i].position - particles[j].position
                normal.normalize_ip()
                relativeVelocity = particles[i].velocity - particles[j].velocity
                dotProduct = relativeVelocity.dot(normal)
                impulse = 2 * dotProduct / (1/particles[i].radius + 1/particles[j].radius)
                particles[i].velocity -= impulse * normal / particles[i].radius
                particles[j].velocity += impulse * normal / particles[j].radius

    screen.fill((0, 0, 0))
    for particle in particles:
        screen.blit(particle.image, particle.rect)
    pygame.display.flip()