import pygame
import sys
import time
import math

screenSize = (400, 200)
ballRadius = 4
fps = 200
# gravity = 160
gravity = 0
collisionMultiplier = 0.8
desnityRadius = 40

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

    def applyForce(self, other):
        # Apply a repulsive force between the two balls
        distance = math.sqrt((self.x - other.x) ** 2 + (self.y - other.y) ** 2)
        forceMultiplier = 2
        forceMagnitude = forceMultiplier * (((self.radius + other.radius) / distance) ** 2)
        # Apply the force so that they are pushed away from each other using trigonometry
        self.vx += forceMagnitude * math.cos(math.atan2(self.y - other.y, self.x - other.x))
        self.vy += forceMagnitude * math.sin(math.atan2(self.y - other.y, self.x - other.x))
        other.vx -= forceMagnitude * math.cos(math.atan2(self.y - other.y, self.x - other.x))
        other.vy -= forceMagnitude * math.sin(math.atan2(self.y - other.y, self.x - other.x))

            
pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Fluid Simulation")
clock = pygame.time.Clock()

balls = []
ballPadding = 60
xCount = 20
yCount = 10
for x in range(xCount):
    for y in range(yCount):
        balls.append(Ball(ballPadding + x * (screenSize[0] - ballPadding * 2) / (xCount - 1), ballPadding + y * (screenSize[1] - ballPadding * 2) / (yCount - 1), 0, 0))

# densityTimer = 0
while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()
        if event.type == pygame.KEYDOWN:
            if event.key == pygame.K_d:
                desnityRadius += 10
            if event.key == pygame.K_a:
                desnityRadius -= 10
                if desnityRadius < 0:
                    desnityRadius = 0

    screen.fill((0, 0, 0))

    for ball in balls:
        ball.move(1 / fps)
        for other in balls:
            if ball != other:
                ball.applyForce(other)
        ball.draw(screen)

    # Draw a circle around the mouse showing density
    # pygame.draw.circle(screen, (255, 255, 255), pygame.mouse.get_pos(), desnityRadius, 1)

    # if(densityTimer > 1):
    #     densityTimer = 0
    #     density = 0
    #     for ball in balls:
    #         if(math.sqrt((ball.x - pygame.mouse.get_pos()[0]) ** 2 + (ball.y - pygame.mouse.get_pos()[1]) ** 2) < desnityRadius):
    #             density += 1
    #     # Divide by the area of the circle
    #     density /= desnityRadius ** 2 * math.pi
    #     print(density)

    pygame.display.flip()
    clock.tick(fps)
    # densityTimer += 1 / fps