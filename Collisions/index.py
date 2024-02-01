import pygame
import sys
import time
import math

screenSize = (1200, 600)
ballSize = 4
horizontalAmount = 25
verticalAmount = 15
fps = 200
horizontalCells = 12
verticalCells = 6
# Accesed like this: x: 2, y: 4 balls[2][4]
balls = [[[] for j in range(verticalCells)] for i in range(horizontalCells)]

class Ball:
    def __init__(self, x, y, v, a, radius):
        self.x = x
        self.y = y
        self.v = v
        self.a = a # Angle in degrees
        self.vx = self.v * math.cos(math.radians(self.a))
        self.vy = self.v * math.sin(math.radians(self.a))
        self.radius = radius
        # Mass is the area of the ball
        self.mass = self.radius ** 2 * 3.14

    def draw(self, screen):
        pygame.draw.circle(screen, (255, 255, 255), (self.x, self.y), self.radius)

    def move(self, dt):
        # Get the current cell
        currentCell = getCell(self.x, self.y)

        # Move the ball
        self.x += self.vx * dt
        self.y += self.vy * dt

        # Apply border collision
        velocityChanged = False
        if self.x < self.radius:
            self.vx = abs(self.vx)
            velocityChanged = True
        elif self.x > screenSize[0] - self.radius:
            self.vx = -abs(self.vx)
            velocityChanged = True
        if self.y < self.radius:
            self.vy = abs(self.vy)
            velocityChanged = True
        elif self.y > screenSize[1] - self.radius:
            self.vy = -abs(self.vy)
            velocityChanged = True

        # Calculate the new angle if the velocity changed from a border collision
        if velocityChanged:
            self.a = math.degrees(math.atan2(self.vy, self.vx))

        # If the cell changed, remove the ball from the previous cell and add it to the new one
        newCell = getCell(self.x, self.y)
        if currentCell != newCell:
            balls[currentCell[0]][currentCell[1]].remove(self)
            balls[newCell[0]][newCell[1]].append(self)
    
    def collide(self, other):
        distance = ((self.x - other.x) ** 2 + (self.y - other.y) ** 2) ** 0.5
        if distance <= self.radius + other.radius:
            originalVx = self.vx
            originalVy = self.vy
            contactAngle = math.degrees(math.atan2(other.y - self.y, other.x - self.x))
            contactAngleRad = math.radians(contactAngle)
            contactAngleCos = math.cos(contactAngleRad)
            contactAngleSin = math.sin(contactAngleRad)
            contactAngle90 = contactAngle + 90
            contactAngle90Rad = math.radians(contactAngle90)
            contactAngle90Cos = math.cos(contactAngle90Rad)
            contactAngle90Sin = math.sin(contactAngle90Rad)

            # Apply direct collision
            self.vx = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * contactAngleCos + self.v * math.sin(math.radians(self.a - contactAngle)) * contactAngle90Cos
            self.vy = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * contactAngleSin + self.v * math.sin(math.radians(self.a - contactAngle)) * contactAngle90Sin
            other.vx = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVx * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * contactAngleCos + other.v * math.sin(math.radians(other.a - contactAngle)) * contactAngle90Cos
            other.vy = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVy * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * contactAngleSin + other.v * math.sin(math.radians(other.a - contactAngle)) * contactAngle90Sin
            self.a = math.degrees(math.atan2(self.vy, self.vx))
            other.a = math.degrees(math.atan2(other.vy, other.vx))
            self.v = (self.vx ** 2 + self.vy ** 2) ** 0.5
            other.v = (other.vx ** 2 + other.vy ** 2)  ** 0.5

            # If the balls are overlapping, move them apart
            if distance < self.radius + other.radius:
                self.x -= (self.radius + other.radius - distance) * math.cos(math.radians(contactAngle))
                self.y -= (self.radius + other.radius - distance) * math.sin(math.radians(contactAngle))
                other.x += (self.radius + other.radius - distance) * math.cos(math.radians(contactAngle))
                other.y += (self.radius + other.radius - distance) * math.sin(math.radians(contactAngle))
            return True
        return False

def getCell(x, y):
    x = int(min(max(x // (screenSize[0] / horizontalCells), 0), horizontalCells - 1))
    y = int(min(max(y // (screenSize[1] / verticalCells), 0), verticalCells - 1))
    return (x, y)
            
# balls = [Ball((screenSize[0] - ballSize * 2) * i / horizontalAmount + ballSize, (screenSize[1] - ballSize * 2) * j / verticalAmount + ballSize, 0, 0, ballSize) for i in range(horizontalAmount) for j in range(verticalAmount)]

for i in range(horizontalAmount):
    for j in range(verticalAmount):
        ballPos = (screenSize[0] - ballSize * 2) * i / horizontalAmount + ballSize, (screenSize[1] - ballSize * 2) * j / verticalAmount + ballSize
        cell = getCell(ballPos[0], ballPos[1])
        balls[cell[0]][cell[1]].append(Ball(ballPos[0], ballPos[1], 0, 0, ballSize))
movingBall = Ball(1160, 560, 750, -135, 20)
movingBallCell = getCell(movingBall.x, movingBall.y)
balls[movingBallCell[0]][movingBallCell[1]].append(movingBall)

pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Ball Collisions")
clock = pygame.time.Clock()

fpsTimer = time.time()
while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()

    screen.fill((0, 0, 0))

    for xCells in balls:
        for yCells in xCells:
            for ball in yCells:
                ball.draw(screen)
                ball.move(1 / fps)

    for x in range(horizontalCells):
        for y in range(verticalCells):
            for ball in balls[x][y]:
                # Check for collisions with the balls in the same cell or the adjacent cells
                for i in range(-1, 2):
                    for j in range(-1, 2):
                        if x + i >= 0 and x + i < horizontalCells and y + j >= 0 and y + j < verticalCells:
                            for otherBall in balls[x + i][y + j]:
                                if ball != otherBall:
                                    ball.collide(otherBall)

    pygame.display.flip()
    clock.tick(fps)
    if time.time() - fpsTimer >= 1:
        print(clock.get_fps())
        fpsTimer = time.time()