import pygame
import sys
import time
import math
import random

screenSize = (1200, 600)
ballSize = 10
horizontalAmount = 25 // 2 * 0
verticalAmount = 12 // 2 * 0
fps = 80
horizontalCells = 24 # 48
verticalCells = 12 # 24
gravity = 0 # 200
repulsionForce = 25000000
balls = []
ballIndexKey = [[-1, -1]for i in range(horizontalCells * verticalCells)]

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
        # Apply gravity
        self.vy += gravity * dt
        self.v = (self.vx ** 2 + self.vy ** 2) ** 0.5

        # Move the ball
        self.x += self.vx * dt
        self.y += self.vy * dt

        # Apply border collision
        velocityChanged = False
        if self.x < self.radius:
            self.vx = abs(self.vx)
            self.x = self.radius
            velocityChanged = True
        elif self.x > screenSize[0] - self.radius:
            self.vx = -abs(self.vx)
            self.x = screenSize[0] - self.radius
            velocityChanged = True
        if self.y < self.radius:
            self.vy = abs(self.vy)
            self.y = self.radius
            velocityChanged = True
        elif self.y > screenSize[1] - self.radius:
            self.vy = -abs(self.vy)
            self.y = screenSize[1] - self.radius
            velocityChanged = True

        # Calculate the new angle if the velocity changed from a border collision
        if velocityChanged:
            self.a = math.degrees(math.atan2(self.vy, self.vx))

        # If the cell changed, remove the ball from the previous cell and add it to the new one
    
    def collide(self, other):
        distance = ((self.x - other.x) ** 2 + (self.y - other.y) ** 2) ** 0.5
        if distance <= self.radius + other.radius:
            originalVx = self.vx
            originalVy = self.vy
            contactAngle = math.degrees(math.atan2(other.y - self.y, other.x - self.x))
            contactAngleRad = math.radians(contactAngle)
            contactAngleCos = math.cos(contactAngleRad)
            contactAngleSin = math.sin(contactAngleRad)
            # cos(x + 90) = -sin(x)
            # sin(x + 90) = cos(x)
            contactAngle90Cos = -contactAngleSin
            contactAngle90Sin = contactAngleCos

            # Apply direct collision
            self.vx = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * contactAngleCos + self.v * math.sin(math.radians(self.a - contactAngle)) * contactAngle90Cos
            self.vy = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * contactAngleSin + self.v * math.sin(math.radians(self.a - contactAngle)) * contactAngle90Sin
            other.vx = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVx * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * contactAngleCos + other.v * math.sin(math.radians(other.a - contactAngle)) * contactAngle90Cos
            other.vy = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVy * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * contactAngleSin + other.v * math.sin(math.radians(other.a - contactAngle)) * contactAngle90Sin

            # # Apply repulsion force (probably cancels out. for example, a hitting b cancels b hitting a)
            # repulForce = ((self.radius + other.radius) ** 2) / (distance ** 0.5) * repulsionForce
            # self.vx += repulForce * math.cos(contactAngleRad) / self.mass
            # self.vy += repulForce * math.sin(contactAngleRad) / self.mass
            # self.vx -= repulForce * math.cos(contactAngleRad) / other.mass
            # self.vy -= repulForce * math.sin(contactAngleRad) / other.mass

            # Update the velocity
            self.a = math.degrees(math.atan2(self.vy, self.vx))
            other.a = math.degrees(math.atan2(other.vy, other.vx))
            self.v = (self.vx ** 2 + self.vy ** 2) ** 0.5
            other.v = (other.vx ** 2 + other.vy ** 2)  ** 0.5

            # If the balls are overlapping, move them apart
            if distance < self.radius + other.radius:
                distanceToMove = (self.radius + other.radius - distance)
                # bigger mass == less movement
                self.x -= distanceToMove * math.cos(math.radians(contactAngle)) * other.mass / (self.mass + other.mass)
                self.y -= distanceToMove * math.sin(math.radians(contactAngle)) * other.mass / (self.mass + other.mass)
                other.x += distanceToMove * math.cos(math.radians(contactAngle)) * self.mass / (self.mass + other.mass)
                other.y += distanceToMove * math.sin(math.radians(contactAngle)) * self.mass / (self.mass + other.mass)
            return True
        return False
    
    def getCell(self):
        x = int(min(max(self.x // (screenSize[0] / horizontalCells), 0), horizontalCells - 1))
        y = int(min(max(self.y // (screenSize[1] / verticalCells), 0), verticalCells - 1))
        return (x, y)
    
    def getCellId(self):
        return self.getCell()[0] + self.getCell()[1] * horizontalCells

for i in range(horizontalAmount):
    for j in range(verticalAmount):
        ballPos = (screenSize[0] - ballSize * 2) * i / horizontalAmount + ballSize, (screenSize[1] - ballSize * 2) * j / verticalAmount + ballSize
        randomVelocities = random.randint(0, 100)
        randomAngle = random.randint(0, 360)
        balls.append(Ball(ballPos[0], ballPos[1], randomVelocities, randomAngle, ballSize))

balls.append(Ball(100, 100, 100, 0, 20))
balls.append(Ball(600, 100, 100, 180, 20))
# balls.append(Ball(100, 100, 200, 45, 40))

pygame.init()
screen = pygame.display.set_mode(screenSize)
pygame.display.set_caption("Ball Collisions")
clock = pygame.time.Clock()

# Use a binary search to get the first index of the ball with the target cell id
def binarySearchBallIndexFirst(arr, targetCellId, start=0, end=len(balls) - 1):
    loops = 0
    while start <= end:
        loops += 1
        mid = (start + end) // 2
        if arr[mid].getCellId() == targetCellId:
            # Get the first index with the same cell id
            for i in range(mid, -1, -1):
                if arr[i].getCellId() != targetCellId:
                    return i + 1
            return 0
        elif arr[mid].getCellId() < targetCellId:
            start = mid + 1
        else:
            end = mid - 1
    return -1

# Use a binary search to get the last index of the ball with the target cell id
def binarySearchBallIndexLast(arr, targetCellId, start=0, end=len(balls) - 1):
    loops = 0
    while start <= end:
        loops += 1
        mid = (start + end) // 2
        if arr[mid].getCellId() == targetCellId:
            # Get the last index with the same cell id
            for i in range(mid, len(arr)):
                if arr[i].getCellId() != targetCellId:
                    return i - 1
            return len(arr) - 1
        elif arr[mid].getCellId() < targetCellId:
            start = mid + 1
        else:
            end = mid - 1
    return -1

# Insertion sort the balls (possibly can use binary sort to make this even faster)
def sortBalls():
    for i in range(1, len(balls)):
        key = balls[i]
        keyId = key.getCellId()
        j = i - 1
        while j >= 0 and balls[j].getCellId() > keyId:
            balls[j + 1] = balls[j]
            j -= 1
        balls[j + 1] = key
    # Update the ball index key
    startIndex = 0
    for i in range(horizontalCells * verticalCells):
        foundIndexStart = binarySearchBallIndexFirst(balls, i, start=startIndex)
        foundIndexEnd = binarySearchBallIndexLast(balls, i, start=startIndex)
        if foundIndexStart!= -1:
            startIndex = foundIndexStart
        ballIndexKey[i] = [foundIndexStart, foundIndexEnd]

fpsTimer = time.time()
while True:
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            sys.exit()

    screen.fill((0, 0, 0))

    # Sort the balls by cell id
    sortBalls()

    # Move and draw the balls
    for ball in balls:
        ball.draw(screen)
        ball.move(1 / fps)

    # Check for collisions in the current cell and the adjacent cells
    for cellX in range(horizontalCells):
        for cellY in range(verticalCells):
            cellId = cellX + cellY * horizontalCells
            for ballIndex in range(ballIndexKey[cellId][0], ballIndexKey[cellId][1] + 1):
                for j in range(-1, 2):
                    for k in range(-1, 2):
                        if cellX + j >= 0 and cellX + j < horizontalCells and cellY + k >= 0 and cellY + k < verticalCells:
                            newCellId = cellX + j + (cellY + k) * horizontalCells
                            for otherBallIndex in range(ballIndexKey[newCellId][0], ballIndexKey[newCellId][1] + 1):
                                # There are no balls in this cell
                                if(ballIndex == -1 or otherBallIndex == -1):
                                    continue
                                if ballIndex != otherBallIndex:
                                    balls[ballIndex].collide(balls[otherBallIndex])

    pygame.display.flip()
    clock.tick(fps)
    if time.time() - fpsTimer >= 1:
        totalKineticEnergy = 0
        for ball in balls:
            totalKineticEnergy += ball.mass * ball.v * ball.v
        print(str(clock.get_fps()) + "\t" + str(totalKineticEnergy))
        fpsTimer = time.time()