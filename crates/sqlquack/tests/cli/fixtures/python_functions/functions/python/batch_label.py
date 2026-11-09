from sqlquack import udf


@udf(arguments={"sku": "VARCHAR"}, returns="VARCHAR", vectorized=True)
def main(sku):
    return None
