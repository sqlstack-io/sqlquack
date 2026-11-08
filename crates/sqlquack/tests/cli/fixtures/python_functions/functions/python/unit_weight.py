from sqlquack import udf


@udf(arguments={"sku": "VARCHAR"}, returns="FLOAT")
def main(sku):
    return None
