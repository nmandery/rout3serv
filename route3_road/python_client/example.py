from shapely.geometry import Point
from datetime import datetime

from route3_client import Server

disturbance = Point(10.86, 48.38).buffer(0.015)
target_points = [
    Point(10.903, 48.348263),
    Point(10.86698055267334, 48.465011068126444),
]

server = Server()
t_start = datetime.now()
stats = server.analyze_disturbance_of_population_movement(disturbance, 5000.0, target_points, num_destinations_to_reach=3)
print(f"took {datetime.now() - t_start}")
print(f"id: {stats.id}")
print(f"population within disturbance: {stats.population_within_disturbance}")
print(stats.dataframe)
