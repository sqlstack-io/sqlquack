from sqlquack import udf


@udf(arguments={"sku": "VARCHAR"}, returns="NUMBER(38,0)")
def main(sku):
    return None
