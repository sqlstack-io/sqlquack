from sqlquack import udf


@udf(arguments={}, returns="NUMBER(38,0)")
def main():
    return None
