// ==== FUN_00455048 @ 00455048 ====

void FUN_00455048(longlong param_1,uint param_2,HMENU param_3,HMENU param_4,undefined4 *param_5)

{
  undefined4 *puVar1;
  bool bVar2;
  undefined7 extraout_var;
  
  puVar1 = param_5;
  *param_5 = 0;
  param_5 = (undefined4 *)0x0;
  if (*(longlong *)(param_1 + 0x7d38) != 0) {
    param_5 = (undefined4 *)FUN_004ee938(*(tagSIZE *)(param_1 + 8),param_2,param_3,param_4);
    if (param_5 != (undefined4 *)0x0) {
      *puVar1 = 1;
    }
  }
  bVar2 = FUN_004ea728(param_1 + 0xc20,param_2,(ulonglong)param_3,param_4,&param_5);
  if ((int)CONCAT71(extraout_var,bVar2) != 0) {
    *puVar1 = 1;
  }
  return;
}


