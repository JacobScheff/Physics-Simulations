import pygame
import sys
import time
import math

screenSize = (1200, 600)
ballRadius = 10
fps = 200
gravity = 160
collisionMultiplier = 0.8

class Ball:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy
        self.radius = ballRadius

    def draw(self, screen):
        pygame.draw.circle(screen, (255, 255, 255), (self.x, self.y), ballRadius)

    def move(self, dt):
        # Apply gravity
        self.vy += gravity * dt

        # If the ball is going to hit the wall, reverse the velocity
        if self.x < self.radius or self.x > screenSize[0] - self.radius:
            self.vx *= -collisionMultiplier
        if self.y < self.radius or self.y > screenSize[1] - self.radius:
            self.vy *= -collisionMultiplier

        # Move the ball
        self.x += self.vx * dt
        self.y += self.vy * dt
            
pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Fluid Simulation")
clock = pygame.time.Clock()

balls = []
ballPadding = 30
xCount = 20
yCount = 10
for x in range(xCount):
    for y in range(yCount):
        balls.append(Ball(ballPadding + x * (screenSize[0] - ballPadding * 2) / (xCount - 1), ballPadding + y * (screenSize[1] - ballPadding * 2) / (yCount - 1), 0, 0))

while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()

    screen.fill((0, 0, 0))

    for ball in balls:
        ball.move(1 / fps)
        ball.draw(screen)

    pygame.display.flip()
    clock.tick(fps)