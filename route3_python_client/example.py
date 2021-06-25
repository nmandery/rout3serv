from shapely.geometry import Point
from route3_client import Server

p1 = Point(8.9, 49.43)
target_points = [
    Point(8.92, 49.412),
    Point(8.88, 49.2),
]

server = Server()
server.analyze_disturbance(p1, 8000.0, target_points)
